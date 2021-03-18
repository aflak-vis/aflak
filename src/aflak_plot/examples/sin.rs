#[macro_use]
extern crate imgui;
extern crate aflak_imgui_glium_support as support;
extern crate aflak_plot;
extern crate ndarray;

use aflak_plot::{
    plot::{self, UiImage1d},
    AxisTransform,
};
use imgui::{Condition, Window};

use std::f32;
use std::path::PathBuf;

fn main() {
    let config = support::AppConfig {
        title: "Example sin.rs".to_owned(),
        ini_filename: Some(PathBuf::from("sin.ini")),
        ..Default::default()
    };
    let mut state = plot::State::default();

    const MAX: f32 = 4.0 * f32::consts::PI;
    let sin = ndarray::Array1::linspace(0.0, MAX, 100).mapv_into(f32::sin);
    let system = support::init(config.clone());
    system.main_loop(config, move |ui, _, _| {
        Window::new(im_str!("Sin"))
            .size([430.0, 450.0], Condition::FirstUseEver)
            .build(ui, || {
                ui.image1d(
                    &sin,
                    "sin(x)",
                    "m",
                    Some(&AxisTransform::new("x", "rad", |x| x / MAX)),
                    &mut state,
                )
                .expect("Image1d failed");
            });
        true
    })
}
