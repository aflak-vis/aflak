use std::{error, fmt};

use glium;

#[derive(Debug)]
pub enum Error {
    Msg(&'static str),
    Glium(glium::texture::TextureCreationError),
}

impl From<&'static str> for Error {
    fn from(s: &'static str) -> Error {
        Error::Msg(s)
    }
}

impl From<glium::texture::TextureCreationError> for Error {
    fn from(e: glium::texture::TextureCreationError) -> Error {
        Error::Glium(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Msg(s) => s.fmt(f),
            Error::Glium(e) => write!(f, "Glium back-end error: {}", e),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Msg(s) => s,
            Error::Glium(ref e) => e.description(),
        }
    }
}
