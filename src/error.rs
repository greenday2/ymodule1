use std::fmt;
use std::io;

#[derive(Debug)]
pub enum Error {
    InvalidFormat(String),
    MissingArgument(String),
    UnknownArgument(String),
    InvalidData(String),
    Io { source: io::Error, context: String },
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Error::Parse(msg) => write!(f, "Parse error: {}", msg),
            Error::Io { source, context } => write!(f, "{}: {}", source, context),
            Error::InvalidFormat(msg) => write!(f, "{}", msg),
            Error::InvalidData(msg) => write!(f, "{}", msg),
            Error::MissingArgument(msg) => write!(f, "{}", msg),
            Error::UnknownArgument(msg) => write!(f, "{}", msg),
        }
    }
}

impl Error {
    pub fn make_io_error(source: io::Error, context: &str) -> Self {
        Error::Io {
            source,
            context: context.to_string(),
        }
    }
}
