use std::fmt;

#[derive(Debug)]
pub enum Error {
    ParseError(String),
    InvalidFormat(String),
    MissingArgument(String),
    UnknownArgument(String),
    SysError {
        source: Box<dyn std::error::Error>,
        context: String,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ParseError(msg) => write!(f, "ParseError: {}", msg),
            Error::SysError { source, context } => write!(f, "{}: {}", source, context),
            Error::InvalidFormat(msg) => write!(f, "{}", msg),
            Error::MissingArgument(msg) => write!(f, "{}", msg),
            Error::UnknownArgument(msg) => write!(f, "{}", msg),
        }
    }
}

impl Error {
    pub fn make_sys_error(source: Box<dyn std::error::Error>, context: &str) -> Self {
        Error::SysError {
            source,
            context: context.to_string(),
        }
    }
}
