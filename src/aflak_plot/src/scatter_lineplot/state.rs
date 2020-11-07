use imgui::Ui;
use implot::{Plot, PlotScatter, PlotUi};
use ndarray::{ArrayBase, Axis, Data, IxDyn};

use super::interactions::Interactions;
use super::AxisTransform;
use super::Error;

/// Current state of a plot UI.
#[derive(Debug)]
pub struct State {
    offset: [f32; 2],
    zoom: [f32; 2],
    mouse_pos: [f32; 2],
    interactions: Interactions,
}

impl Default for State {
    fn default() -> Self {
        use std::f32;
        State {
            offset: [0.0, 0.0],
            zoom: [1.0, 1.0],
            mouse_pos: [f32::NAN, f32::NAN],
            interactions: Interactions::new(),
        }
    }
}

impl State {
    pub(crate) fn plot<S, FX, FY>(
        &mut self,
        ui: &Ui,
        image: &ArrayBase<S, IxDyn>,
        plot_ui: &PlotUi,
        xaxis: Option<&AxisTransform<FX>>,
        yaxis: Option<&AxisTransform<FY>>,
    ) -> Result<(), Error>
    where
        S: Data<Elem = f64>,
        FX: Fn(f32) -> f32,
        FY: Fn(f32) -> f32,
    {
        let content_width = ui.window_content_region_width();
        Plot::new("Simple scatter plot")
            .size(content_width, 300.0)
            .build(&plot_ui, || {
                let x_positions = image.index_axis(Axis(1), 0).slice(s![0..100]).to_vec();
                let y_positions = image.index_axis(Axis(1), 1).slice(s![0..100]).to_vec();
                PlotScatter::new("legend label").plot(&x_positions, &y_positions);
            });
        Ok(())
    }
}
