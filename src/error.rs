use std::error;
use std::fmt;
use std::io;
use std::path::PathBuf;

/// An error raised by the application's own file handling.
#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Parse(String),
    NotFound(PathBuf),
    Invalid(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Parse(message) => write!(f, "parse error: {message}"),
            Self::NotFound(path) => write!(f, "not found: {}", path.display()),
            Self::Invalid(message) => write!(f, "invalid: {message}"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Parse(_) | Self::NotFound(_) | Self::Invalid(_) => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
