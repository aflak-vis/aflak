extern crate aflak_imgui_glium_support as support;
extern crate aflak_plot;
extern crate glium;
extern crate imgui;
extern crate ndarray;

use aflak_plot::three::{self, UiImage3d};
use imgui::Window;
use std::path::PathBuf;

fn main() {
    let config = support::AppConfig {
        title: "Example".to_owned(),
        ini_filename: Some(PathBuf::from("three.ini")),
        ..Default::default()
    };
    let texture_id = imgui::TextureId::from(1);

    let d = 32 as isize;
    let mut image = ndarray::Array3::<f32>::zeros((d as usize, d as usize, d as usize));
    for i in 0..d {
        for j in 0..d {
            for k in 0..d {
                image[[i as usize, j as usize, k as usize]] = 8f32
                    - ((i as f32 - d as f32 / 2f32 + 0.5f32).powi(2)
                        + (j as f32 - d as f32 / 2f32 + 0.5f32).powi(2)
                        + (k as f32 - d as f32 / 2f32 + 0.5f32).powi(2))
                        / (d as f32 / 2f32).powi(2);
            }
        }
    }
    let image = image.into_dyn();
    let events_loop = glium::glutin::event_loop::EventLoop::new();
    let wb = glium::glutin::window::WindowBuilder::new().with_visible(false);
    let cb = glium::glutin::ContextBuilder::new().with_vsync(true);
    let display = glium::Display::new(wb, cb, &events_loop).unwrap();
    let mut state = three::State::default();
    state.display = Some(display);

    support::init(config).main_loop(move |ui, gl_ctx, textures| {
        Window::new(format!("Three")).build(ui, || {
            ui.image3d(&image, texture_id, textures, gl_ctx, &mut state);
        });
        true
    });
}
