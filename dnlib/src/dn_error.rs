use std::error::Error;
use std::io;
use std::fmt;

#[derive(Debug)]
pub enum DnLibError {
    // Errors from external libraries...
    Io(io::Error),
    Walk(walkdir::Error),

    // Errors raised by us...
    InvalidInterestingFile(String),
}

impl Error for DnLibError {
    fn description(&self) -> &str {
        match *self {
            DnLibError::Io(ref err) => err.description(),
            DnLibError::Walk(ref err) => err.description(),
            DnLibError::InvalidInterestingFile(ref s) => s.as_str(),
            //DnLibError::Csv(ref err) => err.description(),
        }
    }
}

impl fmt::Display for DnLibError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DnLibError::Io(ref err) => err.fmt(f),
            DnLibError::Walk(ref err) => err.fmt(f),
            DnLibError::InvalidInterestingFile(ref s) => write!(f, "{}", s),
            //DnLibError::Csv(ref err) => err.fmt(f),
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

// impl From<csv::Error> for DnLibError {
//     fn from(err: csv::Error) -> DnLibError {
//         DnLibError::Csv(err)
//     }
// }

pub type DnLibResult<T> = std::result::Result<T, DnLibError>;
