use imgui::Ui;
use implot::{push_style_var_i32, Marker, Plot, PlotScatter, PlotUi, StyleVar};
use ndarray::{ArrayBase, Axis, Data, Ix2};

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
        image: &ArrayBase<S, Ix2>,
        plot_ui: &PlotUi,
        horizontal_axis: Option<&AxisTransform<FX>>,
        vertical_axis: Option<&AxisTransform<FY>>,
        size: [f32; 2],
    ) -> Result<(), Error>
    where
        S: Data<Elem = f32>,
        FX: Fn(f32) -> f32,
        FY: Fn(f32) -> f32,
    {
        if image.dim().1 != 3 {
            ui.text(format!(
                "Dimension of scatter data must be (N, 3), data is {:?}",
                image.dim()
            ));
            Err(Error::Msg(""))
        } else {
            let mut haxisname = "";
            let mut vaxisname = "";
            let mut haxisunit = "";
            let mut vaxisunit = "";
            if let Some(haxistrans) = horizontal_axis {
                haxisname = haxistrans.label();
                haxisunit = haxistrans.unit();
            }
            if let Some(vaxistrans) = vertical_axis {
                vaxisname = vaxistrans.label();
                vaxisunit = vaxistrans.unit();
            }
            let haxislabel = format!("{} ({})", haxisname, haxisunit);
            let vaxislabel = format!("{} ({})", vaxisname, vaxisunit);
            let size = [size[0] - 15.0, size[1] - 15.0];
            let (data_points, attributes) = image.dim();
            Plot::new("Simple scatter plot")
                .size(size[0], size[1])
                .x_label(&haxislabel)
                .y_label(&vaxislabel)
                .build(&plot_ui, || {
                    let mut image_data = Vec::with_capacity(data_points * attributes);
                    for d in image.axis_iter(Axis(0)) {
                        image_data.push((d[0], d[1], d[2]));
                    }
                    image_data.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());
                    for i in 0..3 {
                        let subdata = image_data
                            .iter()
                            .filter(|d| d.2 == (i as f32 + 1.0))
                            .collect::<Vec<_>>();
                        let data_points = subdata.len();
                        let mut data = Vec::with_capacity(data_points * attributes);
                        for (x, y, class) in subdata {
                            data.push(x);
                            data.push(y);
                            data.push(class);
                        }
                        let image =
                            ndarray::Array::from_shape_vec(vec![data_points, attributes], data)
                                .unwrap();
                        let x_positions = image
                            .index_axis(Axis(1), 0)
                            .slice(s![0..data_points])
                            .to_vec();
                        let y_positions = image
                            .index_axis(Axis(1), 1)
                            .slice(s![0..data_points])
                            .to_vec();
                        let x_positions = x_positions.iter().map(|n| **n as f64).collect();
                        let y_positions = y_positions.iter().map(|n| **n as f64).collect();
                        if i == 0 {
                            let marker_choice =
                                push_style_var_i32(&StyleVar::Marker, Marker::Circle as i32);
                            PlotScatter::new("legend label 1").plot(&x_positions, &y_positions);
                            marker_choice.pop();
                        } else if i == 1 {
                            let marker_choice =
                                push_style_var_i32(&StyleVar::Marker, Marker::Diamond as i32);
                            PlotScatter::new("legend label 2").plot(&x_positions, &y_positions);
                            marker_choice.pop();
                        } else if i == 2 {
                            let marker_choice =
                                push_style_var_i32(&StyleVar::Marker, Marker::Cross as i32);
                            PlotScatter::new("legend label 3").plot(&x_positions, &y_positions);
                            marker_choice.pop();
                        }
                    }
                });
            Ok(())
        }
    }
}
