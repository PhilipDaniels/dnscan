use std::error::Error;
use std::{io, fmt};

#[derive(Debug)]
pub enum DnLibError {
    // An IO error occurred, for example when reading a file.
    IoError(String),
    // A directory walk error occurred. This may happen when scanning
    // the input directory for interesting files.
    WalkError(String),
    // A Git error occurred.
    GitError(String),
}

impl Error for DnLibError {
    fn description(&self) -> &str {
        "description is deprecated, use Display() instead"
    }
}

impl fmt::Display for DnLibError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DnLibError::IoError(ref s) => write!(f, "{}", s),
            DnLibError::WalkError(ref s) => write!(f, "{}", s),
            DnLibError::GitError(ref s) => write!(f, "{}", s),
        }
    }
}

impl From<io::Error> for DnLibError {
    fn from(err: io::Error) -> DnLibError {
        DnLibError::IoError(err.to_string())
    }
}

impl From<walkdir::Error> for DnLibError {
    fn from(err: walkdir::Error) -> DnLibError {
        DnLibError::WalkError(err.to_string())
    }
}

impl From<git2::Error> for DnLibError {
    fn from(err: git2::Error) -> DnLibError {
        DnLibError::GitError(err.to_string())
    }
}

pub type DnLibResult<T> = std::result::Result<T, DnLibError>;
