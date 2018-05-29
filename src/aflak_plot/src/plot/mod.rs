mod state;

use imgui::Ui;
use ndarray::Array1;

use super::interactions;
use super::lims;
use super::ticks;
use super::util;
use super::Error;

pub use self::state::State;

pub trait UiImage1d {
    fn image1d(&self, image: &Array1<f32>, state: &mut State) -> Result<(), Error>;
}

impl<'ui> UiImage1d for Ui<'ui> {
    fn image1d(&self, image: &Array1<f32>, state: &mut State) -> Result<(), Error> {
        let p = self.get_cursor_screen_pos();
        let size = self.get_window_size();
        state.plot(self, image, p, size)
    }
}
