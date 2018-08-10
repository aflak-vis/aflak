extern crate glium;
#[macro_use]
extern crate aflak_imgui as imgui;
extern crate aflak_imgui_glium_renderer as imgui_glium_renderer;
extern crate ndarray;
extern crate ui_image2d;

use glium::backend::Facade;
use imgui_glium_renderer::{AppConfig, AppContext};
use ui_image2d::UiImage2d;

fn main() {
    let config = AppConfig {
        ini_filename: Some(imgui::ImString::new("gradient.ini")),
        ..Default::default()
    };
    let mut app = AppContext::init("Example".to_owned(), config).unwrap();
    let gl_ctx = app.get_context().clone();
    let mut state = ui_image2d::State::default();
    app.run(|ui| {
        let image_data = {
            const WIDTH: usize = 100;
            const HEIGHT: usize = 100;
            let mut image_data = Vec::with_capacity(WIDTH * HEIGHT);
            for j in 0..WIDTH {
                for i in 0..HEIGHT {
                    image_data.push((i + j) as f32);
                }
            }
            ndarray::Array2::from_shape_vec((WIDTH, HEIGHT), image_data).unwrap()
        };
        ui.window(im_str!("Gradient")).build(|| {
            ui.image2d(&gl_ctx, im_str!("Gradient"), &image_data, &mut state)
                .expect("Image2d failed");
        });
        true
    }).unwrap();
}
