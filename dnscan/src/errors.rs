use std::io;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum AnalysisError {
    // Errors from external libraries...
    Io(io::Error),
    // Errors raised by us...
    InvalidInterestingFile(String),
    //Regular(ErrorKind),
    //Custom(String)
}

impl Error for AnalysisError {
    fn description(&self) -> &str {
        match *self {
            AnalysisError::Io(ref err) => err.description(),
            AnalysisError::InvalidInterestingFile(ref s) => s.as_str(),
        }
    }
}

impl fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AnalysisError::Io(ref err) => err.fmt(f),
            AnalysisError::InvalidInterestingFile(ref s) => write!(f, "{}", s),
        }
    }
}

impl From<io::Error> for AnalysisError {
    fn from(err: io::Error) -> AnalysisError {
        AnalysisError::Io(err)
    }
}

pub type AnalysisResult<T> = std::result::Result<T, AnalysisError>;
