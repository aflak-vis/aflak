extern crate glium;
#[macro_use]
extern crate imgui;
extern crate imgui_glium_renderer;
extern crate ndarray;

mod err;
mod hist;
mod image;
mod lims;
mod lut;
mod state;

pub use err::Error;
pub use state::{State, Value, ValueId, ValueIter};

use glium::backend::Facade;
use imgui::{ImStr, Ui};
use ndarray::Array2;

impl<'ui> UiImage2d for Ui<'ui> {
    /// Show image given as input. `name` is used as an ID to register the
    /// provided image as an OpenGL texture in [`Ui`].
    ///
    /// # Example
    ///
    /// ```rust
    /// #[macro_use] extern crate imgui;
    /// extern crate imgui_glium_renderer;
    /// extern crate ndarray;
    /// extern crate ui_image2d;
    ///
    /// use imgui::Ui;
    /// use imgui_glium_renderer::AppContext;
    /// use ndarray::Array2;
    /// use ui_image2d::UiImage2d;
    ///
    /// fn run(ui: &Ui, ctx: &AppContext) -> Result<(), ui_image2d::Error> {
    ///     let data = Array2::eye(10);
    ///     let mut state = ui_image2d::State::default();
    ///     ui.image2d(ctx, im_str!("Show my image!"), &data, &mut state)
    /// }
    /// ```
    fn image2d<F>(
        &self,
        ctx: &F,
        name: &ImStr,
        image: &Array2<f32>,
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

/// Implementation of a UI to visualize a 2D image with ImGui and OpenGL
pub trait UiImage2d {
    fn image2d<F>(
        &self,
        ctx: &F,
        name: &ImStr,
        image: &Array2<f32>,
        state: &mut State,
    ) -> Result<(), Error>
    where
        F: Facade;
}
