mod hist;
mod image;
mod lut;
mod state;

pub use self::state::State;

use std::borrow::Borrow;

use glium::{backend::Facade, Texture2d};
use imgui::{self, ImTexture, Ui};
use ndarray::ArrayD;

use err::Error;
use interactions;
use ticks;
use util;

use super::AxisTransform;

pub type Textures = imgui::Textures<Texture2d>;

impl<'ui> UiImage2d for Ui<'ui> {
    /// Show image given as input. `name` is used as an ID to register the
    /// provided image as an OpenGL texture in [`Ui`].
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// #[macro_use] extern crate imgui;
    /// extern crate aflak_imgui_glium_support as support;
    /// extern crate ndarray;
    /// extern crate aflak_plot;
    ///
    /// use std::time::Instant;
    ///
    /// use imgui::{ImTexture, Ui};
    /// use ndarray::Array2;
    /// use aflak_plot::{
    ///     imshow::{self, UiImage2d},
    ///     AxisTransform,
    /// };
    ///
    /// fn main() {
    ///     let mut state = imshow::State::default();
    ///     support::run(Default::default(), |ui, gl_ctx, textures| {
    ///         let texture_id = ImTexture::from(1);
    ///         if state.image_created_on().is_none() {
    ///             let data = Array2::eye(10).into_dimensionality().unwrap();
    ///             if let Err(e) = state.set_image(data, Instant::now(), gl_ctx, texture_id, textures) {
    ///                 eprintln!("{:?}", e);
    ///                 return false;
    ///             }
    ///         }
    ///         if let Err(e) = ui.image2d(
    ///             gl_ctx,
    ///             textures,
    ///             texture_id,
    ///             "<unit>",
    ///             AxisTransform::none(),
    ///             AxisTransform::none(),
    ///             &mut state,
    ///         ) {
    ///             eprintln!("{:?}", e);
    ///             false
    ///         } else {
    ///             true
    ///         }
    ///     }).unwrap()
    /// }
    /// ```
    fn image2d<F, FX, FY, I>(
        &self,
        ctx: &F,
        textures: &mut Textures,
        texture_id: ImTexture,
        vunit: &str,
        xaxis: Option<&AxisTransform<FX>>,
        yaxis: Option<&AxisTransform<FY>>,
        state: &mut State<I>,
    ) -> Result<(), Error>
    where
        F: Facade,
        FX: Fn(f32) -> f32,
        FY: Fn(f32) -> f32,
        I: Borrow<ArrayD<f32>>,
    {
        let window_pos = self.get_window_pos();
        let cursor_pos = self.get_cursor_screen_pos();
        let window_size = self.get_window_size();
        const HIST_WIDTH: f32 = 40.0;
        const BAR_WIDTH: f32 = 20.0;

        const RIGHT_PADDING: f32 = 100.0;
        let image_max_size = (
            // Add right padding so that ticks and labels on the right fits
            window_size.0 - HIST_WIDTH - BAR_WIDTH - RIGHT_PADDING,
            window_size.1 - (cursor_pos.1 - window_pos.1),
        );
        let ([p, size], x_label_height) =
            state.show_image(self, texture_id, vunit, xaxis, yaxis, image_max_size)?;

        state.show_hist(
            self,
            [p.0 + size.0 as f32, p.1],
            [HIST_WIDTH, size.1 as f32],
        );
        let lut_bar_updated = state.show_bar(
            self,
            [p.0 + size.0 as f32 + HIST_WIDTH, p.1],
            [BAR_WIDTH, size.1 as f32],
        );
        if lut_bar_updated {
            state
                .image()
                .update_texture(ctx, texture_id, textures, &state.lut)?;
        }

        self.set_cursor_screen_pos([p.0, p.1 + size.1 + x_label_height]);
        state.show_roi_selector(self);

        Ok(())
    }
}

/// Implementation of a UI to visualize a 2D image with ImGui and OpenGL
pub trait UiImage2d {
    fn image2d<F, FX, FY, I>(
        &self,
        ctx: &F,
        textures: &mut Textures,
        texture_id: ImTexture,
        vunit: &str,
        xaxis: Option<&AxisTransform<FX>>,
        yaxis: Option<&AxisTransform<FY>>,
        state: &mut State<I>,
    ) -> Result<(), Error>
    where
        F: Facade,
        FX: Fn(f32) -> f32,
        FY: Fn(f32) -> f32,
        I: Borrow<ArrayD<f32>>;
}
