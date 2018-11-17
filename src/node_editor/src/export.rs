use std::error;
use std::fmt;
use std::io;

use cake;
use ron::{de, ser};

#[derive(Debug)]
pub enum ExportError {
    SerializationError(ser::Error),
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

impl From<ser::Error> for ExportError {
    fn from(serial_error: ser::Error) -> Self {
        ExportError::SerializationError(serial_error)
    }
}

#[derive(Debug)]
pub enum ImportError<E> {
    DSTError(cake::ImportError<E>),
    DeserializationError(de::Error),
    IOError(io::Error),
}

impl<E: fmt::Display> fmt::Display for ImportError<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ImportError::DSTError(ref e) => write!(f, "Error while building DST! {}", e),
            ImportError::DeserializationError(ref e) => write!(f, "Deserialization error! {}", e),
            ImportError::IOError(ref e) => write!(f, "I/O error! {}", e),
        }
    }
}

impl<E: fmt::Display + fmt::Debug> error::Error for ImportError<E> {
    fn description(&self) -> &'static str {
        "ImportError"
    }
}

impl<E> From<io::Error> for ImportError<E> {
    fn from(io_error: io::Error) -> Self {
        ImportError::IOError(io_error)
    }
}

impl<E> From<de::Error> for ImportError<E> {
    fn from(deserial_error: de::Error) -> Self {
        ImportError::DeserializationError(deserial_error)
    }
}

impl<E> From<cake::ImportError<E>> for ImportError<E> {
    fn from(e: cake::ImportError<E>) -> Self {
        ImportError::DSTError(e)
    }
}
