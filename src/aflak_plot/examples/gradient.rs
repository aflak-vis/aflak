extern crate glium;
#[macro_use]
extern crate imgui;
extern crate aflak_imgui_glium_support as support;
extern crate aflak_plot;
extern crate ndarray;

use std::time::Instant;

use aflak_plot::{
    imshow::{self, UiImage2d},
    AxisTransform,
};

fn main() {
    let config = support::AppConfig {
        title: "Example".to_owned(),
        ini_filename: Some(imgui::ImString::new("gradient.ini")),
        ..Default::default()
    };
    let mut state = imshow::State::default();
    let texture_id = imgui::ImTexture::from(1);

    support::run(config, |ui, gl_ctx, textures| {
        if state.image_created_on().is_none() {
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
            state
                .set_image(image_data, Instant::now(), gl_ctx, texture_id, textures)
                .unwrap();
        }
        ui.window(im_str!("Gradient")).build(|| {
            ui.image2d(
                gl_ctx,
                textures,
                texture_id,
                "gradient value",
                Some(AxisTransform::new("X Axis", |x| x)),
                Some(AxisTransform::new("Y Axis", |y| y)),
                &mut state,
            ).expect("Image2d failed");
        });
        true
    }).unwrap();
}
