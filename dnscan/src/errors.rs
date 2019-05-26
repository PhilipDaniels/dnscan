use csv;
use std::error::Error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum AnalysisError {
    // Errors from external libraries...
    DnLib(dnlib::DnLibError),
    Io(io::Error),
    Csv(csv::Error),
    // Errors raised by us...
    //Regular(ErrorKind),
    //Custom(String)
}

impl Error for AnalysisError {
    fn description(&self) -> &str {
        match *self {
            AnalysisError::DnLib(ref err) => err.description(),
            AnalysisError::Io(ref err) => err.description(),
            AnalysisError::Csv(ref err) => err.description(),
        }
    }
}

impl fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AnalysisError::DnLib(ref err) => err.fmt(f),
            AnalysisError::Io(ref err) => err.fmt(f),
            AnalysisError::Csv(ref err) => err.fmt(f),
        }
    }
}

impl From<dnlib::DnLibError> for AnalysisError {
    fn from(err: dnlib::DnLibError) -> AnalysisError {
        AnalysisError::DnLib(err)
    }
}

impl From<io::Error> for AnalysisError {
    fn from(err: io::Error) -> AnalysisError {
        AnalysisError::Io(err)
    }
}

impl From<csv::Error> for AnalysisError {
    fn from(err: csv::Error) -> AnalysisError {
        AnalysisError::Csv(err)
    }
}

pub type AnalysisResult<T> = std::result::Result<T, AnalysisError>;
