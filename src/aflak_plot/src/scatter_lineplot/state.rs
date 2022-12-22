use super::interactions::{
    FinedGrainedROI, Interaction, InteractionId, InteractionIterMut, Interactions, ValueIter,
};
use super::AxisTransform;
use super::Error;
use crate::imshow::cake::{OutputId, TransformIdx};
use imgui::{ImString, MenuItem, MouseButton, Ui, Window};
use implot::{
    get_plot_limits, get_plot_mouse_position, is_plot_hovered, push_style_var_i32, Condition,
    ImPlotLimits, ImPlotPoint, Marker, Plot, PlotLine, PlotScatter, PlotUi, StyleVar, YAxisChoice,
};
use ndarray::{ArrayBase, Axis, Data, Ix2};
use std::collections::HashMap;
type EditableValues = HashMap<InteractionId, TransformIdx>;

/// Current state of a plot UI.
#[derive(Debug)]
pub struct State {
    //offset: [f32; 2],
    //zoom: [f32; 2],
    //mouse_pos: [f32; 2],
    interactions: Interactions,
    plot_limits: [f64; 4],
    limit_changed: bool,
    datapoints: HashMap<Vec<bool>, (Vec<f64>, Vec<f64>, Vec<Vec<f32>>)>,
    //class: Vec<usize>,
    pub show_graph_editor: bool,
    show_graph: Vec<bool>,
    expr: Vec<String>,
    bind: Vec<String>,
    graph_points: Vec<Vec<f64>>,
    graph_removing: Option<usize>,
    pub show_all_point: bool,
    pub editor_changed: bool,
    pub roi_cnt: usize,
}

impl Default for State {
    fn default() -> Self {
        State {
            //offset: [0.0, 0.0],
            //zoom: [1.0, 1.0],
            //mouse_pos: [f32::NAN, f32::NAN],
            interactions: Interactions::new(),
            plot_limits: [0.0, 1.0, 0.0, 1.0],
            limit_changed: true,
            datapoints: HashMap::new(),
            //class: vec![],
            show_graph_editor: false,
            show_graph: vec![false],
            expr: vec![String::from("")],
            bind: vec![String::from("")],
            graph_points: vec![vec![]],
            graph_removing: None,
            show_all_point: true,
            editor_changed: false,
            roi_cnt: 0,
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
                .size(size)
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
        _copying: &mut Option<(InteractionId, TransformIdx)>,
        store: &mut EditableValues,
        attaching: &mut Option<(OutputId, TransformIdx, usize)>,
        outputid: OutputId,
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
        let index_size = image.dim().0 - 3;
        let mut indexes = Vec::new();
        for i in 0..index_size {
            indexes.push(image.slice(s![i + 3, ..]).to_vec());
        }
        let mut datapoints = Vec::new();
        for i in 0..xaxis.len() {
            let mut index = Vec::new();
            for j in 0..index_size {
                index.push(indexes[j][i]);
            }
            datapoints.push((xaxis[i], yaxis[i], distances[i], index));
        }
        let content_width = ui.window_content_region_width();
        let mut plot_limits: Option<ImPlotLimits> = None;
        let mut hover_pos_plot: Option<ImPlotPoint> = None;
        let mut graph_changed = false;
        if let Some((o, t_idx, kind)) = *attaching {
            if o == outputid && kind == 2 {
                let mut already_insert = false;
                for d in store.iter() {
                    if *d.1 == t_idx {
                        already_insert = true;
                        break;
                    }
                }
                if !already_insert {
                    let new = Interaction::FinedGrainedROI(FinedGrainedROI::new(self.roi_cnt));
                    self.interactions.insert(new);
                    store.insert(self.interactions.id(), t_idx);
                    self.roi_cnt += 1;
                } else {
                    eprintln!("{:?} is already bound", t_idx)
                }
                *attaching = None;
            }
        }
        if self.show_graph_editor {
            Window::new(&ImString::new(format!("Graph Editor")))
                .size([300.0, 500.0], Condition::Appearing)
                .resizable(false)
                .build(ui, || {
                    for i in 0..self.show_graph.len() {
                        let p = ui.cursor_screen_pos();
                        let mut out_expr = String::with_capacity(1024);
                        out_expr.push_str(&self.expr[i]);
                        ui.text("y = ");
                        ui.set_cursor_screen_pos([p[0] + 40.0, p[1]]);
                        let changed = ui.input_text(&format!("expr_{}", i), &mut out_expr).build();
                        if changed {
                            graph_changed = true;
                            self.expr[i] = out_expr;
                        }
                        let p = ui.cursor_screen_pos();
                        let mut out_bind = String::with_capacity(1024);
                        out_bind.push_str(&self.bind[i]);
                        ui.text("bind:");
                        ui.set_cursor_screen_pos([p[0] + 40.0, p[1]]);
                        let changed = ui.input_text(&format!("bind_{}", i), &mut out_bind).build();
                        if changed {
                            graph_changed = true;
                            self.bind[i] = out_bind;
                        }
                        if ui.checkbox(&format!("Show graph {}", i), &mut self.show_graph[i]) {
                            graph_changed = true;
                        }
                        if ui.button(&format!("Delete Function {}", i)) {
                            self.graph_removing = Some(i);
                        }
                    }
                    if ui.button(format!("Add Function")) {
                        self.show_graph.push(false);
                        self.expr.push(String::from(""));
                        self.bind.push(String::from(""));
                        self.graph_points.push(vec![]);
                    }
                });
        }
        let is_any_graph_show = {
            let mut res = false;
            for t in self.show_graph.iter() {
                res |= t;
            }
            res
        };
        for i in 0..self.show_graph.len() {
            if let Ok(expr) = &self.expr[i].parse::<meval::Expr>() {
                if let Ok(func) = expr.clone().bind(&self.bind[i]) {
                    self.graph_points[i] = (0..300 + 1)
                        .map(|i| {
                            let minx = self.plot_limits[0];
                            let maxx = self.plot_limits[1];
                            let computex = minx + (maxx - minx) / 300.0 * i as f64;
                            func(computex)
                        })
                        .collect();
                } else {
                    self.graph_points[i].clear();
                }
            } else {
                self.graph_points[i].clear();
            }
        }
        let is_roi_changed = {
            let mut res = false;
            for (_id, interaction) in self.interactions.iter_mut() {
                match interaction {
                    Interaction::FinedGrainedROI(FinedGrainedROI {
                        id: _,
                        pixels: _,
                        changed,
                    }) => {
                        res |= *changed;
                    }
                    _ => {}
                }
            }
            res
        };
        if self.limit_changed || graph_changed || is_roi_changed || self.editor_changed {
            self.datapoints.clear();
            let (xmin, xmax, ymin, ymax) = (
                self.plot_limits[0],
                self.plot_limits[1],
                self.plot_limits[2],
                self.plot_limits[3],
            );
            let xstep = ((xmax - xmin) / content_width as f64 * 5.0) as f64;
            let ystep = ((ymax - ymin) / size[1] as f64 * 5.0) as f64;
            let standard_distance = xstep * xstep + ystep * ystep;
            if !self.show_all_point {
                datapoints.retain(|data| {
                    xmin <= data.0 as f64
                        && data.0 as f64 <= xmax
                        && ymin <= data.1 as f64
                        && data.1 as f64 <= ymax
                });
                datapoints.dedup_by(|x, y| {
                    ((x.0 - y.0).abs() as f64) < xstep && ((x.1 - y.1).abs() as f64) < ystep
                });
            }
            loop {
                if datapoints.first() == None {
                    break;
                }
                let first = datapoints.first().unwrap().clone();
                if !self.show_all_point {
                    datapoints.retain(|x| {
                        x.2 > first.2 * 2.0
                            || ((x.0 - first.0) * (x.0 - first.0)
                                + (x.1 - first.1) * (x.1 - first.1))
                                as f64
                                > standard_distance
                    });
                }
                let key = {
                    if is_any_graph_show {
                        let mut t = Vec::new();
                        for i in 0..self.show_graph.len() {
                            if self.show_graph[i] {
                                if let Ok(expr) = &self.expr[i].parse::<meval::Expr>() {
                                    if let Ok(func) = expr.clone().bind(&self.bind[i]) {
                                        if func(first.0 as f64) > first.1 as f64 {
                                            t.push(true);
                                        } else {
                                            t.push(false);
                                        }
                                    }
                                }
                            } else {
                                t.push(false);
                            }
                        }
                        t
                    } else {
                        let mut t = Vec::new();
                        for (_id, interaction) in self.interactions.iter_mut() {
                            match interaction {
                                Interaction::FinedGrainedROI(FinedGrainedROI {
                                    id: _,
                                    pixels,
                                    changed: _,
                                }) => {
                                    let idx = first.3.clone();
                                    if idx.len() != 2 {
                                        t.push(false);
                                    }
                                    for pixel in pixels {
                                        if idx[0] as usize == pixel.0 && idx[1] as usize == pixel.1
                                        {
                                            t.push(true);
                                        }
                                    }
                                }
                                _ => {}
                            };
                        }
                        t
                    }
                };
                let mut inserted = false;
                for (k, v) in self.datapoints.iter_mut() {
                    if *k == key {
                        v.0.push(first.0 as f64);
                        v.1.push(first.1 as f64);
                        v.2.push(first.3.clone());
                        inserted = true;
                        break;
                    }
                }
                if !inserted {
                    self.datapoints.insert(
                        key,
                        (vec![first.0 as f64], vec![first.1 as f64], vec![first.3]),
                    );
                }
                if self.show_all_point {
                    datapoints.remove(0);
                }
            }
            self.limit_changed = false;
            self.editor_changed = false;
        }
        if let Some(key) = self.graph_removing {
            self.show_graph.remove(key);
            self.expr.remove(key);
            self.bind.remove(key);
            self.graph_points.remove(key);
            self.graph_removing = None;
        }
        Plot::new("Simple Plot")
            .size([content_width, size[1]])
            .x_label(&haxislabel)
            .y_label(&vaxislabel)
            .x_limits(
                (self.plot_limits[0], self.plot_limits[1]),
                Condition::FirstUseEver,
            )
            .y_limits(
                (self.plot_limits[2], self.plot_limits[3]),
                YAxisChoice::First,
                Condition::FirstUseEver,
            )
            .build(plot_ui, || {
                plot_limits = Some(get_plot_limits(None));
                if is_plot_hovered() {
                    if ui.is_mouse_clicked(MouseButton::Right) {
                        ui.open_popup(format!("add-node"));
                    }
                    hover_pos_plot = Some(get_plot_mouse_position(None));
                }
                ui.popup(format!("add-node"), || {
                    let mut counter = 0;
                    for (_, (_, _, idx)) in self.datapoints.iter() {
                        if MenuItem::new(&format!("Add data points {}", counter)).build(ui) {
                            let mut datavec: Vec<(usize, usize)> = Vec::new();
                            for d in idx {
                                if d.len() == 2 {
                                    datavec.push((d[0] as usize, d[1] as usize));
                                }
                            }
                            let new = Interaction::FinedGrainedROI(FinedGrainedROI::from_vec(
                                counter, datavec,
                            ));
                            self.interactions.insert(new);
                            self.roi_cnt += 1;
                        }
                        counter += 1;
                    }
                });
                if let Some(ImPlotPoint { x, y }) = hover_pos_plot {
                    let mut mindistance =
                        (self.plot_limits[1] - self.plot_limits[0]) * 5.0 / content_width as f64;
                    let mut minindex = vec![];
                    for d in self.datapoints.values() {
                        for idx in 0..d.0.len() {
                            if (d.0[idx] - x) * (d.0[idx] - x) + (d.1[idx] - y) * (d.1[idx] - y)
                                < mindistance * mindistance
                            {
                                minindex.clear();
                                minindex = d.2[idx].clone();
                                mindistance = (d.0[idx] - x) * (d.0[idx] - x)
                                    + (d.1[idx] - y) * (d.1[idx] - y);
                            }
                        }
                    }
                    if !minindex.is_empty() {
                        ui.tooltip_text(format!("This datapoint comes from pixel {:?}", minindex));
                    }
                }
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
                let mut counter = 0;
                for (_, (xaxis, yaxis, _)) in self.datapoints.iter() {
                    PlotScatter::new(format!("data points {}", counter).as_str())
                        .plot(&xaxis, &yaxis);
                    counter += 1;
                }

                (0..self.show_graph.len())
                    .map(|i| {
                        if self.show_graph[i] {
                            let x_positions: Vec<_> = (0..300 + 1)
                                .map(|i| {
                                    let minx = self.plot_limits[0];
                                    let maxx = self.plot_limits[1];
                                    let computex = minx + (maxx - minx) / 300.0 * i as f64;
                                    computex
                                })
                                .collect();
                            PlotLine::new(format!("graph {}", i).as_str())
                                .plot(&x_positions, &self.graph_points[i]);
                        }
                    })
                    .count();
            });
        Ok(())
    }

    pub fn stored_values(&self) -> ValueIter {
        self.interactions.value_iter()
    }

    pub fn stored_values_mut(&mut self) -> InteractionIterMut {
        self.interactions.iter_mut()
    }
}
