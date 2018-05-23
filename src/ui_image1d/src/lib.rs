extern crate imgui;

use imgui::Ui;

pub trait UiImage1d {
    fn image1d(&self, image: &[f32], state: &mut State) -> Result<(), Error>;
}

impl<'ui> UiImage1d for Ui<'ui> {
    fn image1d(&self, _image: &[f32], _state: &mut State) -> Result<(), Error> {
        Ok(())
    }
}

#[derive(Debug)]
pub enum Error {}

pub struct State {}

impl Default for State {
    fn default() -> Self {
        Self {}
    }
}
