extern crate glium;
#[macro_use]
extern crate imgui;
extern crate aflak_imgui_glium_support as support;
extern crate aflak_plot;
extern crate ndarray;

use aflak_plot::{
    imshow::{self, UiImage2d},
    AxisTransform,
};

fn main() {
    let config = support::AppConfig {
        title: "Example peak.rs".to_owned(),
        ini_filename: Some(imgui::ImString::new("peak.ini")),
        ..Default::default()
    };
    let mut state = imshow::State::default();
    let image_data = {
        const WIDTH: usize = 200;
        const HEIGHT: usize = 100;
        ndarray::Array2::from_shape_fn((HEIGHT, WIDTH), |(j, i)| {
            use std::f32;
            let i = i as isize;
            let j = j as isize;
            let width = WIDTH as isize;
            let height = HEIGHT as isize;
            let sin = f32::sin((i - width / 3) as f32 / WIDTH as f32 * 2.0 * f32::consts::PI);
            let cos = f32::cos((j - height / 3) as f32 / HEIGHT as f32 * 2.0 * f32::consts::PI);
            f32::exp(sin * sin + cos * cos)
        })
    };

    support::run(config, |ui, gl_ctx, textures| {
        ui.window(im_str!("Peak")).build(|| {
            ui.image2d(
                gl_ctx,
                textures,
                imgui::ImTexture::from(1),
                &image_data,
                "exp(sin(x)^2 + cos(y)^2)",
                Some(AxisTransform::new("X Axis", |x| x)),
                Some(AxisTransform::new("Y Axis", |y| y)),
                &mut state,
            ).expect("Image2d failed");
        });
        true
    }).unwrap();
}
