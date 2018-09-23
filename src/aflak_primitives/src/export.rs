use std::error::Error;
use std::fmt;
use std::io;
use std::path::Path;

use fitrs::{Fits, Hdu};

use super::IOValue;

impl IOValue {
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<Fits, ExportError> {
        Fits::create(
            path,
            match self {
                IOValue::Image1d(arr) => Hdu::new(
                    arr.shape(),
                    arr.as_slice()
                        .expect("Could not get slice out of array")
                        .to_owned(),
                ),
                IOValue::Image2d(arr) => Hdu::new(
                    arr.shape(),
                    arr.as_slice()
                        .expect("Could not get slice out of array")
                        .to_owned(),
                ),
                _ => {
                    return Err(ExportError::NotImplemented(
                        "Can only save Image1d and Image2d",
                    ))
                }
            },
        ).map_err(ExportError::IOError)
    }
}

#[derive(Debug)]
pub enum ExportError {
    IOError(io::Error),
    NotImplemented(&'static str),
}

impl fmt::Display for ExportError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ExportError::IOError(e) => write!(fmt, "{}", e),
            ExportError::NotImplemented(e) => write!(fmt, "Not implemented: {}", e),
        }
    }
}

impl Error for ExportError {}
