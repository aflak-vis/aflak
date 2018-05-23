#[macro_use]
extern crate imgui;

use imgui::Ui;

pub trait UiImage1d {
    fn image1d(&self, image: &[f32], state: &mut State) -> Result<(), Error>;
}

impl<'ui> UiImage1d for Ui<'ui> {
    fn image1d(&self, image: &[f32], _state: &mut State) -> Result<(), Error> {
        self.plot_lines(im_str!("Plot"), image)
            .graph_size([0.0, 400.0])
            .build();
        self.text(format!("{} data points", image.len()));
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
