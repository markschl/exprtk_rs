
use std::fmt;
use std::error::Error;
use std::ffi::CStr;

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
                self.line_no,
                self.column_no,
                self.token_value,
                self.message
            )
        } else {
            write!(f, "Parse error at {}: {}", self.token_value, self.message)
        }
    }
}

impl Error for ParseError {
    fn description(&self) -> &str {
        "expression parsing error"
    }
}



#[derive(Debug, PartialEq)]
pub struct InvalidName(pub String);


impl fmt::Display for InvalidName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Invalid variable name: '{}'", self.0)
    }
}

impl Error for InvalidName {
    fn description(&self) -> &str {
        "invalid variable name"
    }
}

pub(super) unsafe fn get_err(c_parser: *mut CParser) -> ParseError {
    let e: &CParseError = &*parser_error(c_parser);
    if e.is_err {
        let err_out = ParseError {
            kind: ParseErrorKind::from_i32(e.mode as i32).unwrap_or_else(|| panic!(
                "Unknown ParseErrorKind enum variant: {}",
                e.mode
            )),
            token_type: string_from_ptr!(e.token_type),
            token_value: string_from_ptr!(e.token_value),
            message: string_from_ptr!(e.diagnostic),
            line: string_from_ptr!(e.error_line),
            line_no: e.line_no as usize,
            column_no: e.column_no as usize,
        };
        parser_error_free(e as *const CParseError);
        err_out
    } else {
        panic!("Compiler notified about error, but there is none.")
    }
}
