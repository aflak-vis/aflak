extern crate aflak_imgui_glium_support as support;
extern crate aflak_plot;
extern crate glium;
extern crate imgui;
extern crate implot;
#[macro_use(s)]
extern crate ndarray;

use aflak_plot::imshow::cake::OutputId;
use std::collections::HashMap;
use std::path::PathBuf;

use aflak_plot::{
    scatter_lineplot::{State, UiScatter},
    AxisTransform,
};
use imgui::{im_str, Condition, Ui, Window};
use implot::{
    push_style_var_f32, push_style_var_i32, Context, Marker, Plot, PlotScatter, PlotUi, StyleVar,
};

fn main() {
    let config = support::AppConfig {
        title: "Example".to_owned(),
        ini_filename: Some(PathBuf::from("scatter.ini")),
        ..Default::default()
    };
    let mut state = State::default();
    let system = support::init(config.clone());

    let plotcontext = Context::create();

    system.main_loop(config, move |_run, ui, _, _| {
        let plot_ui = plotcontext.get_plot_ui();
        let image_data = {
            const WIDTH: usize = 10;
            const HEIGHT: usize = 10;
            let mut image_data = Vec::with_capacity(WIDTH * HEIGHT);
            for j in 0..WIDTH {
                for i in 0..HEIGHT {
                    image_data.push(j as f32 * 0.1);
                    image_data.push(i as f32 * 0.1);
                    image_data.push(if i > 5 { 1.0 } else { 2.0 });
                }
            }
            ndarray::Array::from_shape_vec(vec![WIDTH * HEIGHT, 3], image_data).unwrap()
        };
        let image_data = image_data.slice(s![.., ..]);
        Window::new(im_str!("Scatter plots example"))
            .size([430.0, 450.0], Condition::FirstUseEver)
            .build(ui, || {
                ui.scatter(
                    &image_data,
                    &plot_ui,
                    Some(&AxisTransform::new("X Axis", "m", |x| x)),
                    Some(&AxisTransform::new("Y Axis", "m", |y| y)),
                    &mut state,
                    &mut None,
                    &mut HashMap::new(),
                    &mut None,
                    OutputId::new(0),
                )
                .expect("Scatter failed");
            });
        true
    });
}

pub fn show_custom_markers_plot(ui: &Ui, plot_ui: &PlotUi) {
    ui.text(im_str!(
        "This header shows how markers can be used in scatter plots."
    ));
    let content_width = ui.window_content_region_width();
    Plot::new("Multi-marker scatter plot")
        // The size call could also be omitted, though the defaults don't consider window
        // width, which is why we're not doing so here.
        .size(content_width, 300.0)
        .build(plot_ui, || {
            // Change to cross marker for one scatter plot call
            let x_positions = vec![0.1, 0.2, 0.1, 0.5, 0.9];
            let y_positions = vec![0.1, 0.1, 0.3, 0.3, 0.9];
            let markerchoice = push_style_var_i32(&StyleVar::Marker, Marker::Cross as i32);
            PlotScatter::new("legend label 1").plot(&x_positions, &y_positions);
            markerchoice.pop();

            // One can combine things like marker size and markor choice
            let x_positions = vec![0.4, 0.1];
            let y_positions = vec![0.5, 0.3];
            let marker_choice = push_style_var_i32(&StyleVar::Marker, Marker::Diamond as i32);
            let marker_size = push_style_var_f32(&StyleVar::MarkerSize, 12.0);
            PlotScatter::new("legend label 2").plot(&x_positions, &y_positions);

            // TODO(4bb4) check if these have to be in reverse push order. Does not
            // seem to be the case.
            marker_size.pop();
            marker_choice.pop();
        });
}
