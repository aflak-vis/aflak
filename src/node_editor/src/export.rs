use std::error;
use std::fmt;
use std::io;

use crate::cake;
use ron::{de, error as ser_error};

#[derive(Debug)]
pub enum ExportError {
    SerializationError(ser_error::Error),
    IOError(io::Error),
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ExportError::SerializationError(ref e) => write!(f, "Serialization error! {}", e),
            ExportError::IOError(ref e) => write!(f, "I/O error! {}", e),
        }
    }
}

impl error::Error for ExportError {
    fn description(&self) -> &'static str {
        "ExportError"
    }
}

impl From<io::Error> for ExportError {
    fn from(io_error: io::Error) -> Self {
        ExportError::IOError(io_error)
    }
}

impl From<ser_error::Error> for ExportError {
    fn from(serial_error: ser_error::Error) -> Self {
        ExportError::SerializationError(serial_error)
    }
}

#[derive(Debug)]
pub enum ImportError {
    DSTError(cake::ImportError),
    DeserializationError(de::Error),
    IOError(io::Error),
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ImportError::DSTError(ref e) => write!(f, "Error while building DST! {}", e),
            ImportError::DeserializationError(ref e) => write!(f, "Deserialization error! {}", e),
            ImportError::IOError(ref e) => write!(f, "I/O error! {}", e),
        }
    }
}

impl error::Error for ImportError {
    fn description(&self) -> &'static str {
        "ImportError"
    }
}

impl From<io::Error> for ImportError {
    fn from(io_error: io::Error) -> Self {
        ImportError::IOError(io_error)
    }
}

impl From<de::Error> for ImportError {
    fn from(deserial_error: de::Error) -> Self {
        ImportError::DeserializationError(deserial_error)
    }
}

impl From<cake::ImportError> for ImportError {
    fn from(e: cake::ImportError) -> Self {
        ImportError::DSTError(e)
    }
}
