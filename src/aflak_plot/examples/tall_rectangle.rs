extern crate glium;
#[macro_use]
extern crate imgui;
extern crate aflak_imgui_glium_support as support;
extern crate aflak_plot;
extern crate ndarray;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use aflak_plot::{
    imshow::{self, UiImage2d},
    AxisTransform,
};

use imshow::cake::OutputId;

fn main() {
    let config = support::AppConfig {
        title: "Example tall_rectangle.rs".to_owned(),
        ini_filename: Some(PathBuf::from("tall_rectangle.ini")),
        ..Default::default()
    };
    let mut state = imshow::State::default();
    let texture_id = imgui::TextureId::from(1);
    let system = support::init(config.clone());

    support::init(config).main_loop(move |ui, gl_ctx, textures| {
        if state.image_created_on().is_none() {
            let image_data = {
                const WIDTH: usize = 10;
                const HEIGHT: usize = 20;
                let mut image_data = Vec::with_capacity(WIDTH * HEIGHT);
                for _ in 0..HEIGHT {
                    for i in 0..WIDTH {
                        image_data.push(i as f32);
                    }
                }
                ndarray::ArrayD::from_shape_vec(vec![HEIGHT, WIDTH], image_data).unwrap()
            };
            state
                .set_image(image_data, Instant::now(), gl_ctx, texture_id, textures)
                .unwrap();
        }
        imgui::Window::new(im_str!("Tall Rectangle")).build(ui, || {
            ui.image2d(
                gl_ctx,
                textures,
                imgui::TextureId::from(1),
                "pixel",
                Some(&AxisTransform::new("X Axis", "m", |x| x)),
                Some(&AxisTransform::new("Y Axis", "m", |y| y)),
                &mut state,
                &mut None,
                &mut HashMap::new(),
                &mut None,
                OutputId::new(0),
            )
            .expect("Image2d failed");
        });
        true
    });
}
