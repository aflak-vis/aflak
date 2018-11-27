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
        title: "Example tall_rectangle.rs".to_owned(),
        ini_filename: Some(imgui::ImString::new("tall_rectangle.ini")),
        ..Default::default()
    };
    let mut state = imshow::State::default();
    let texture_id = imgui::ImTexture::from(1);

    support::run(config, |ui, gl_ctx, textures| {
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
                ndarray::Array2::from_shape_vec((HEIGHT, WIDTH), image_data).unwrap()
            };
            state
                .set_image(image_data, Instant::now(), gl_ctx, texture_id, textures)
                .unwrap();
        }
        ui.window(im_str!("Tall Rectangle")).build(|| {
            ui.image2d(
                gl_ctx,
                textures,
                imgui::ImTexture::from(1),
                "pixel",
                Some(AxisTransform::new("X Axis", |x| x)),
                Some(AxisTransform::new("Y Axis", |y| y)),
                &mut state,
            ).expect("Image2d failed");
        });
        true
    }).unwrap();
}
