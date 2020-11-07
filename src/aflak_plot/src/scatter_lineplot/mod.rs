//! Draw scatter_plot.
mod state;

use imgui::Ui;
use implot::PlotUi;
use ndarray::{ArrayBase, Data, IxDyn};

use super::interactions;
use super::AxisTransform;
use super::Error;

pub use self::state::State;

/// Implementation of a UI to visualize a 1D image with ImGui using a plot.
pub trait UiScatter {
    fn scatter<S, FX, FY>(
        &self,
        image: &ArrayBase<S, IxDyn>,
        plot_ui: &PlotUi,
        xaxis: Option<&AxisTransform<FX>>,
        yaxis: Option<&AxisTransform<FY>>,
        state: &mut State,
    ) -> Result<(), Error>
    where
        S: Data<Elem = f64>,
        FX: Fn(f32) -> f32,
        FY: Fn(f32) -> f32;
}

impl<'ui> UiScatter for Ui<'ui> {
    /// Draw a plot in the remaining space of the window.
    ///
    /// The mutable reference `state` contains the current state of the user
    /// interaction with the window.
    fn scatter<S, FX, FY>(
        &self,
        image: &ArrayBase<S, IxDyn>,
        plot_ui: &PlotUi,
        xaxis: Option<&AxisTransform<FX>>,
        yaxis: Option<&AxisTransform<FY>>,
        state: &mut State,
    ) -> Result<(), Error>
    where
        S: Data<Elem = f64>,
        FX: Fn(f32) -> f32,
        FY: Fn(f32) -> f32,
    {
        let p = self.cursor_screen_pos();
        let window_pos = self.window_pos();
        let window_size = self.window_size();
        let size = [window_size[0], window_size[1] - (p[1] - window_pos[1])];
        state.plot(self, image, plot_ui, xaxis, yaxis)
    }
}
