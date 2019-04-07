use std::error::Error;
use std::{io, fmt};

#[derive(Debug)]
pub enum DnLibError {
    // Errors from external libraries...
    Io(io::Error),
    Walk(walkdir::Error),
    Git(git2::Error),

    // Errors raised by us...
    InvalidInterestingFile(String),
}

impl Error for DnLibError {
    fn description(&self) -> &str {
        match *self {
            DnLibError::Io(ref err) => err.description(),
            DnLibError::Walk(ref err) => err.description(),
            DnLibError::InvalidInterestingFile(ref s) => s.as_str(),
            DnLibError::Git(ref err) => err.description(),
        }
    }
}

impl fmt::Display for DnLibError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DnLibError::Io(ref err) => err.fmt(f),
            DnLibError::Walk(ref err) => err.fmt(f),
            DnLibError::InvalidInterestingFile(ref s) => write!(f, "{}", s),
            DnLibError::Git(ref err) => err.fmt(f),
        }
    }
}

impl From<io::Error> for DnLibError {
    fn from(err: io::Error) -> DnLibError {
        DnLibError::Io(err)
    }
}

impl From<walkdir::Error> for DnLibError {
    fn from(err: walkdir::Error) -> DnLibError {
        DnLibError::Walk(err)
    }
}

impl From<git2::Error> for DnLibError {
    fn from(err: git2::Error) -> DnLibError {
        DnLibError::Git(err)
    }
}

pub type DnLibResult<T> = std::result::Result<T, DnLibError>;
