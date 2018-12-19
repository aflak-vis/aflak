#[macro_use]
extern crate imgui;
extern crate aflak_imgui_glium_support as support;
extern crate aflak_plot;
extern crate ndarray;

use aflak_plot::{
    imshow::{self, UiImage2d},
    plot::{self, UiImage1d},
    AxisTransform,
};

use std::f32;

fn main() -> Result<(), support::Error> {
    let config = support::AppConfig {
        title: "Example sin.rs".to_owned(),
        ini_filename: Some(imgui::ImString::new("sin.ini")),
        ..Default::default()
    };
    let mut state = plot::State::default();
    let mut state2 = imshow::State::default();
    let texture_id = imgui::ImTexture::from(1);

    const MAX: f32 = 4.0 * f32::consts::PI;
    let sin = ndarray::Array1::linspace(0.0, MAX, 100).mapv_into(f32::sin);

    support::run(config, |ui, gl_ctx, textures| {
        ui.window(im_str!("Sin")).build(|| {
            ui.image1d(
                &sin,
                "sin(x)",
                Some(&AxisTransform::new("x (rad)", |x| x / MAX)),
                &mut state,
            )
            .expect("Image1d failed");
        });
        if state2.image_created_on().is_none() {
            const WIDTH: usize = 200;
            const HEIGHT: usize = 100;
            let image_data = ndarray::Array2::from_shape_fn([HEIGHT, WIDTH], |(j, i)| {
                use std::f32;
                let i = i as isize;
                let j = j as isize;
                let width = WIDTH as isize;
                let height = HEIGHT as isize;
                let sin = f32::sin((i - width / 3) as f32 / WIDTH as f32 * 2.0 * f32::consts::PI);
                let cos = f32::cos((j - height / 3) as f32 / HEIGHT as f32 * 2.0 * f32::consts::PI);
                f32::exp(sin * sin + cos * cos)
            })
            .into_dimensionality()
            .unwrap();
            state2
                .set_image(
                    image_data,
                    ::std::time::Instant::now(),
                    gl_ctx,
                    texture_id,
                    textures,
                )
                .unwrap();
        }
        ui.window(im_str!("Peak")).build(|| {
            ui.image2d(
                gl_ctx,
                textures,
                texture_id,
                "exp(sin(x)^2 + cos(y)^2)",
                Some(&AxisTransform::new("X Axis", |x| x)),
                Some(&AxisTransform::new("Y Axis", |y| y)),
                &mut state2,
            )
            .expect("Image2d failed");
        });
        true
    })
}
