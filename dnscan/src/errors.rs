use std::io;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum AnalysisError {
    // Errors from external libraries...
    Io(io::Error),
    // Errors raised by us...
    //Regular(ErrorKind),
    //Custom(String)
}

impl Error for AnalysisError {
    fn description(&self) -> &str {
        match *self {
            AnalysisError::Io(ref err) => err.description(),
        }
    }
}

impl fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AnalysisError::Io(ref err) => err.fmt(f),
        }
    }
}

impl From<io::Error> for AnalysisError {
    fn from(err: io::Error) -> AnalysisError {
        AnalysisError::Io(err)
    }
}

pub type AnalysisResult<T> = std::result::Result<T, AnalysisError>;
