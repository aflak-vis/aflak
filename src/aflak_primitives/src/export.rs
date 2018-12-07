use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use fitrs::{Fits, Hdu};

use super::IOValue;

impl IOValue {
    pub fn save<P, R>(&self, path: P, r: &R) -> Result<(), ExportError>
    where
        P: AsRef<Path>,
        R: ToString,
    {
        match self {
            IOValue::Integer(i) => write_to_file_as_display(path, i)?,
            IOValue::Float(f) => write_to_file_as_display(path, f)?,
            IOValue::Float2(f2) => write_to_file_as_debug(path, f2)?,
            IOValue::Float3(f3) => write_to_file_as_debug(path, f3)?,
            IOValue::Str(s) => write_to_file_as_display(path, s)?,
            IOValue::Bool(b) => write_to_file_as_display(path, b)?,
            IOValue::Path(p) => write_to_file_as_display(path, &p.to_string_lossy())?,
            IOValue::Fits(_) => return Err(ExportError::NotImplemented("Cannot copy FITS file.")),
            IOValue::Image(arr) => {
                let arr = arr.scalar();
                let mut hdu = Hdu::new(
                    arr.shape(),
                    arr.as_slice()
                        .expect("Could not get slice out of array")
                        .to_owned(),
                );
                hdu.insert("AFLAPROV", r.to_string());
                Fits::create(path, hdu)?;
            }
            IOValue::Map2dTo3dCoords(_) => {
                return Err(ExportError::NotImplemented("Cannot export Map2dTo3dCoords"))
            }
            IOValue::Roi(_) => {
                return Err(ExportError::NotImplemented(
                    "Cannot export region of interest.",
                ))
            }
        }
        Ok(())
    }

    pub fn extension(&self) -> &'static str {
        match self {
            IOValue::Integer(_)
            | IOValue::Float(_)
            | IOValue::Float2(_)
            | IOValue::Float3(_)
            | IOValue::Str(_)
            | IOValue::Bool(_)
            | IOValue::Path(_) => "txt",
            IOValue::Image(_) | IOValue::Fits(_) => "fits",
            _ => "txt",
        }
    }
}

fn write_to_file_as_display<P: AsRef<Path>, T: fmt::Display>(path: P, t: &T) -> io::Result<()> {
    let buf = format!("{}\n", t);
    write_to_file_as_bytes(path, buf.as_bytes())
}
fn write_to_file_as_debug<P: AsRef<Path>, T: fmt::Debug>(path: P, t: &T) -> io::Result<()> {
    let buf = format!("{:?}\n", t);
    write_to_file_as_bytes(path, buf.as_bytes())
}
fn write_to_file_as_bytes<P: AsRef<Path>>(path: P, buf: &[u8]) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(buf)
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

impl Error for ExportError {
    /// description is deprecated. See https://github.com/rust-lang/rust/issues/44842
    /// Implement for compilation to succeed on older compilers.
    fn description(&self) -> &str {
        "ExportError"
    }
}

impl From<io::Error> for ExportError {
    fn from(e: io::Error) -> Self {
        ExportError::IOError(e)
    }
}
