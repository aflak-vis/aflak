extern crate imgui;

use imgui::Ui;

/// Contains interactions with UiImage2d subwindow
pub struct Interaction {}

impl<'ui> UiImage2d for Ui<'ui> {
    fn image2d(&self, _image: &[Vec<f32>]) -> Interaction {
        // TODO
        self.text("Image2d");
        Interaction {}
    }
}

pub trait UiImage2d {
    fn image2d(&self, image: &[Vec<f32>]) -> Interaction;
}
