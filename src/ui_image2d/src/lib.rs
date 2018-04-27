extern crate imgui;

use imgui::Ui;

/// Contains interactions with UiImage2d subwindow
pub struct Interaction {}

impl<'ui> UiImage2d for Ui<'ui> {
    fn image2d<V>(&self, _image: &[V]) -> Interaction
    where
        V: AsRef<[f32]>,
    {
        // TODO
        self.text("Image2d");
        Interaction {}
    }
}

pub trait UiImage2d {
    fn image2d<V>(&self, image: &[V]) -> Interaction
    where
        V: AsRef<[f32]>;
}
