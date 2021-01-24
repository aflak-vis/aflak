//! Draw 2D images.
mod hist;
mod image;
mod lut;
mod state;

pub extern crate aflak_cake as cake;

pub use self::interactions::InteractionId;
pub use self::state::State;

use std::borrow::Borrow;
use std::collections::HashMap;

use self::cake::TransformIdx;
use glium::backend::Facade;
use imgui::{self, TextureId, Ui};
use imgui_glium_renderer::Texture;
use ndarray::ArrayD;

use crate::err::Error;
use crate::interactions;
use crate::ticks;
use crate::util;

use super::AxisTransform;

/// A handle to an OpenGL 2D texture.
pub type Textures = imgui::Textures<Texture>;
type EditabaleValues = HashMap<InteractionId, TransformIdx>;

impl<'ui> UiImage2d for Ui<'ui> {
    /// Show image given as input.
    ///
    /// The mutable reference `state` contains the current state of the user
    /// interaction with the window.
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
    /// use imgui::{TextureId, Ui};
    /// use ndarray::Array2;
    /// use aflak_plot::{
    ///     imshow::{self, UiImage2d},
    ///     AxisTransform,
    /// };
    ///
    /// fn main() {
    ///     let config = support::AppConfig {
    ///         ..Default::default()
    ///     };
    ///     let mut state = imshow::State::default();
    ///     support::init(Default::default()).main_loop(move |ui, gl_ctx, textures| {
    ///         let texture_id = TextureId::from(1);
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
    ///     })
    /// }
    /// ```
    fn image2d<F, FX, FY, I>(
        &self,
        ctx: &F,
        textures: &mut Textures,
        texture_id: TextureId,
        vunit: &str,
        xaxis: Option<&AxisTransform<FX>>,
        yaxis: Option<&AxisTransform<FY>>,
        state: &mut State<I>,
        copying: &mut Option<(InteractionId, TransformIdx)>,
        store: &mut EditabaleValues,
    ) -> Result<(), Error>
    where
        F: Facade,
        FX: Fn(f32) -> f32,
        FY: Fn(f32) -> f32,
        I: Borrow<ArrayD<f32>>,
    {
        let window_pos = self.window_pos();
        let cursor_pos = self.cursor_screen_pos();
        let window_size = self.window_size();
        const HIST_WIDTH: f32 = 40.0;
        const BAR_WIDTH: f32 = 20.0;

        const RIGHT_PADDING: f32 = 100.0;
        let image_max_size = (
            // Add right padding so that ticks and labels on the right fits
            window_size[0] - HIST_WIDTH - BAR_WIDTH - RIGHT_PADDING,
            window_size[1] - (cursor_pos[1] - window_pos[1]),
        );
        let ([p, size], x_label_height) = state.show_image(
            self,
            texture_id,
            vunit,
            xaxis,
            yaxis,
            image_max_size,
            &mut *copying,
            &mut *store,
        )?;

        state.show_hist(self, [p[0] + size[0], p[1]], [HIST_WIDTH, size[1]]);
        let lut_bar_updated = state.show_bar(
            self,
            [p[0] + size[0] + HIST_WIDTH, p[1]],
            [BAR_WIDTH, size[1]],
        );
        if lut_bar_updated {
            state
                .image()
                .update_texture(ctx, texture_id, textures, &state.lut)?;
        }

        self.set_cursor_screen_pos([p[0], p[1] + size[1] + x_label_height]);
        state.show_roi_selector(self);

        Ok(())
    }

    fn color_image<F, FX, FY, I>(
        &self,
        _ctx: &F,
        _textures: &mut Textures,
        texture_id: TextureId,
        vunit: &str,
        xaxis: Option<&AxisTransform<FX>>,
        yaxis: Option<&AxisTransform<FY>>,
        state: &mut State<I>,
        copying: &mut Option<(InteractionId, TransformIdx)>,
        store: &mut EditabaleValues,
    ) -> Result<(), Error>
    where
        F: Facade,
        FX: Fn(f32) -> f32,
        FY: Fn(f32) -> f32,
        I: Borrow<ArrayD<f32>>,
    {
        let window_pos = self.window_pos();
        let cursor_pos = self.cursor_screen_pos();
        let window_size = self.window_size();
        const HIST_WIDTH: f32 = 300.0;
        const BAR_WIDTH: f32 = 20.0;

        const RIGHT_PADDING: f32 = 100.0;
        let image_max_size = (
            // Add right padding so that ticks and labels on the right fits
            window_size[0] - HIST_WIDTH - BAR_WIDTH - RIGHT_PADDING,
            window_size[1] - (cursor_pos[1] - window_pos[1]),
        );
        let ([p, size], x_label_height) = state.show_image(
            self,
            texture_id,
            vunit,
            xaxis,
            yaxis,
            image_max_size,
            &mut *copying,
            &mut *store,
        )?;

        state.show_hist_color(self, [p[0] + size[0], p[1]], [HIST_WIDTH, size[1]]);
        let _lut_bar_updated = state.show_bar(
            self,
            [p[0] + size[0] + HIST_WIDTH, p[1]],
            [BAR_WIDTH, size[1]],
        );
        /*if lut_bar_updated {
            state
                .image()
                .update_texture(ctx, texture_id, textures, &state.lut)?;
        }*/

        self.set_cursor_screen_pos([p[0], p[1] + size[1] + x_label_height]);
        state.show_roi_selector(self);

        Ok(())
    }
}

/// Implementation of a UI to visualize a 2D image with ImGui and OpenGL.
pub trait UiImage2d {
    fn image2d<F, FX, FY, I>(
        &self,
        ctx: &F,
        textures: &mut Textures,
        texture_id: TextureId,
        vunit: &str,
        xaxis: Option<&AxisTransform<FX>>,
        yaxis: Option<&AxisTransform<FY>>,
        state: &mut State<I>,
        copying: &mut Option<(InteractionId, TransformIdx)>,
        store: &mut EditabaleValues,
    ) -> Result<(), Error>
    where
        F: Facade,
        FX: Fn(f32) -> f32,
        FY: Fn(f32) -> f32,
        I: Borrow<ArrayD<f32>>;

    fn color_image<F, FX, FY, I>(
        &self,
        ctx: &F,
        textures: &mut Textures,
        texture_id: TextureId,
        vunit: &str,
        xaxis: Option<&AxisTransform<FX>>,
        yaxis: Option<&AxisTransform<FY>>,
        state: &mut State<I>,
        copying: &mut Option<(InteractionId, TransformIdx)>,
        store: &mut EditabaleValues,
    ) -> Result<(), Error>
    where
        F: Facade,
        FX: Fn(f32) -> f32,
        FY: Fn(f32) -> f32,
        I: Borrow<ArrayD<f32>>;
}
