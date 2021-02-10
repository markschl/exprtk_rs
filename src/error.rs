use std::error::Error;
use std::ffi::CStr;
use std::fmt;

use enum_primitive::FromPrimitive;
use exprtk_sys::*;

pub type ParseResult<T> = Result<T, ParseError>;

#[derive(Debug, PartialEq, Clone)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub token_type: String,
    pub token_value: String,
    pub message: String,
    pub line: String,
    pub line_no: usize,
    pub column_no: usize,
}

impl ParseError {
    pub(super) fn simple_syntax(s: &str, msg: &str) -> Self {
        let s = s.to_string();
        ParseError {
            kind: ParseErrorKind::Syntax,
            token_type: "".to_string(),
            token_value: s,
            message: msg.to_string(),
            line: "".to_string(),
            line_no: 1,
            column_no: 1,
        }
    }

    pub(super) unsafe fn from_c_err(c_parser: *mut CParser) -> Option<Self> {
        let e: &CParseError = &*parser_error(c_parser);
        if e.is_err {
            let err_out = ParseError {
                kind: ParseErrorKind::from_i32(e.mode as i32)
                    .unwrap_or_else(|| panic!("Unknown ParseErrorKind enum variant: {}", e.mode)),
                token_type: string_from_ptr!(e.token_type),
                token_value: string_from_ptr!(e.token_value),
                message: string_from_ptr!(e.diagnostic),
                line: string_from_ptr!(e.error_line),
                line_no: e.line_no as usize,
                column_no: e.column_no as usize,
            };
            parser_error_free(e as *const CParseError);
            Some(err_out)
        } else {
            None
        }
    }
}

enum_from_primitive! {
    #[derive(Debug, PartialEq, Clone)]
    pub enum ParseErrorKind {
        Unknown,
        Syntax,
        Token,
        Numeric,
        Symtab,
        Lexer,
        Helper
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.line_no > 0 || self.column_no > 0 {
            write!(
                f,
                "Parse error at line {}, column {} ({}): {}",
                self.line_no, self.column_no, self.token_value, self.message
            )
        } else {
            write!(f, "Parse error at {}: {}", self.token_value, self.message)
        }
    }
}

impl Error for ParseError {}

#[derive(Debug, PartialEq)]
pub struct InvalidName(pub String);

impl fmt::Display for InvalidName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Invalid variable name: '{}'", self.0)
    }
}

impl Error for InvalidName {}

impl From<InvalidName> for ParseError {
    fn from(e: InvalidName) -> Self {
        ParseError::simple_syntax(&e.0, "Non-ASCII character or null byte found in formula")
    }
}
