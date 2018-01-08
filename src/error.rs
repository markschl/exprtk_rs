
use std::fmt;
use std::error::Error;


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
