extern crate glium;
#[macro_use]
extern crate imgui;
extern crate imgui_glium_support as support;
extern crate ndarray;
extern crate ui_image2d;

use ui_image2d::UiImage2d;

fn main() {
    let config = support::AppConfig {
        title: "Example".to_owned(),
        ini_filename: Some(imgui::ImString::new("gradient.ini")),
        ..Default::default()
    };
    let mut state = ui_image2d::State::default();
    support::run(config, |ui, gl_ctx, textures| {
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
            ui.image2d(
                gl_ctx,
                textures,
                imgui::ImTexture::from(1),
                &image_data,
                &mut state,
            ).expect("Image2d failed");
        });
        true
    }).unwrap();
}
