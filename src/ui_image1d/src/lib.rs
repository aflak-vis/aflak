#[macro_use]
extern crate imgui;

use imgui::Ui;

pub trait UiImage1d {
    fn image1d(&self, image: &[f32], state: &mut State);
}

impl<'ui> UiImage1d for Ui<'ui> {
    fn image1d(&self, image: &[f32], _: &mut State) {
        const PLOT_HEIGHT: f32 = 400.0;
        self.plot_lines(im_str!(""), image)
            .graph_size([0.0, PLOT_HEIGHT])
            .build();
        self.text(format!("{} data points", image.len()));
    }
}

pub struct State;

impl Default for State {
    fn default() -> Self {
        State
    }
}
