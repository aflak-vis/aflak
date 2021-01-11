use imgui::Ui;
use implot::{
    get_plot_limits, push_style_var_i32, Condition, ImPlotLimits, ImPlotRange,
    Marker, Plot, PlotScatter, PlotUi, StyleVar, YAxisChoice,
};
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
    plot_limits: [f64; 4],
    limit_changed: bool,
    xaxis: Vec<f64>,
    yaxis: Vec<f64>,
}

impl Default for State {
    fn default() -> Self {
        use std::f32;
        State {
            offset: [0.0, 0.0],
            zoom: [1.0, 1.0],
            mouse_pos: [f32::NAN, f32::NAN],
            interactions: Interactions::new(),
            plot_limits: [0.0, 1.0, 0.0, 1.0],
            limit_changed: true,
            xaxis: vec![],
            yaxis: vec![],
        }
    }
}

impl State {
    pub(crate) fn _plot<S, FX, FY>(
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
                        let x_positions =
                            x_positions.iter().map(|&&n| n as f64).collect::<Vec<f64>>();
                        let y_positions =
                            y_positions.iter().map(|&&n| n as f64).collect::<Vec<f64>>();
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
    pub(crate) fn simple_plot<S, FX, FY>(
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
        let xaxis = image.slice(s![0, ..]).to_vec();
        let yaxis = image.slice(s![1, ..]).to_vec();
        let distances = image.slice(s![2, ..]).to_vec();
        let mut datapoints = Vec::new();
        for i in 0..xaxis.len() {
            datapoints.push((xaxis[i], yaxis[i], distances[i]));
        }
        let content_width = ui.window_content_region_width();
        let mut plot_limits: Option<ImPlotLimits> = None;
        if self.limit_changed {
            self.xaxis.clear();
            self.yaxis.clear();
            let (xmin, xmax, ymin, ymax) = (
                self.plot_limits[0],
                self.plot_limits[1],
                self.plot_limits[2],
                self.plot_limits[3],
            );
            datapoints.retain(|&data| {
                xmin <= data.0 as f64
                    && data.0 as f64 <= xmax
                    && ymin <= data.1 as f64
                    && data.1 as f64 <= ymax
            });
            let xstep = ((xmax - xmin) / content_width as f64 * 5.0) as f64;
            let ystep = ((ymax - ymin) / size[1] as f64 * 5.0) as f64;
            let standard_distance = xstep * xstep + ystep * ystep;
            datapoints.dedup_by(|x, y| {
                ((x.0 - y.0).abs() as f64) < xstep && ((x.1 - y.1).abs() as f64) < ystep
            });
            loop {
                if datapoints.first() == None {
                    break;
                }
                let first = datapoints.first().unwrap().clone();
                datapoints.retain(|x| {
                    x.2 > first.2 * 2.0
                        || ((x.0 - first.0) * (x.0 - first.0) + (x.1 - first.1) * (x.1 - first.1))
                            as f64
                            > standard_distance
                });
                self.xaxis.push(first.0 as f64);
                self.yaxis.push(first.1 as f64);
            }
            self.limit_changed = false;
        }

        Plot::new("Simple Plot")
            .size(content_width, size[1])
            .x_label(&haxislabel)
            .y_label(&vaxislabel)
            .x_limits(
                &ImPlotRange {
                    Min: self.plot_limits[0],
                    Max: self.plot_limits[1],
                },
                Condition::FirstUseEver,
            )
            .y_limits(
                &ImPlotRange {
                    Min: self.plot_limits[2],
                    Max: self.plot_limits[3],
                },
                YAxisChoice::First,
                Condition::FirstUseEver,
            )
            .build(plot_ui, || {
                plot_limits = Some(get_plot_limits(None));
                if let Some(plot) = plot_limits {
                    if self.plot_limits[0] != plot.X.Min
                        || self.plot_limits[1] != plot.X.Max
                        || self.plot_limits[2] != plot.Y.Min
                        || self.plot_limits[3] != plot.Y.Max
                    {
                        self.plot_limits[0] = plot.X.Min;
                        self.plot_limits[1] = plot.X.Max;
                        self.plot_limits[2] = plot.Y.Min;
                        self.plot_limits[3] = plot.Y.Max;
                        self.limit_changed = true;
                    }
                }
                PlotScatter::new("data point").plot(&self.xaxis, &self.yaxis);
            });
        Ok(())
    }
}
