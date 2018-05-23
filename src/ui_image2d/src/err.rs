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
