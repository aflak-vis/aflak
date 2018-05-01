extern crate glium;
#[macro_use]
extern crate imgui;
extern crate imgui_glium_renderer;

mod err;
mod hist;
mod image;
mod lims;
mod lut;
mod state;

pub use err::Error;
pub use state::State;

use glium::backend::Facade;
use imgui::{ImStr, Ui};

impl<'ui> UiImage2d for Ui<'ui> {
    fn image2d<F>(
        &self,
        ctx: &F,
        name: &ImStr,
        image: &Vec<Vec<f32>>,
        state: &mut State,
    ) -> Result<(), Error>
    where
        F: Facade,
    {
        state.vmin = lims::get_vmin(image)?;
        state.vmax = lims::get_vmax(image)?;

        let [p, size] = state.show_image(self, ctx, name, image)?;

        const HIST_WIDTH: f32 = 40.0;
        const BAR_WIDTH: f32 = 20.0;
        state.show_hist(
            self,
            [p.0 + size.0 as f32, p.1],
            [HIST_WIDTH, size.1 as f32],
            image,
        );
        state.show_bar(
            self,
            [p.0 + size.0 as f32 + HIST_WIDTH, p.1],
            [BAR_WIDTH, size.1 as f32],
        );

        Ok(())
    }
}

pub trait UiImage2d {
    fn image2d<F>(
        &self,
        ctx: &F,
        name: &ImStr,
        image: &Vec<Vec<f32>>,
        state: &mut State,
    ) -> Result<(), Error>
    where
        F: Facade;
}
