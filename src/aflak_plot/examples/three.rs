extern crate glium;
#[macro_use]
extern crate imgui;
extern crate aflak_imgui_glium_support as support;
extern crate aflak_plot;
extern crate ndarray;

use aflak_plot::three::UiImage3d;

fn main() {
    let config = support::AppConfig {
        title: "Example".to_owned(),
        ini_filename: Some(imgui::ImString::new("three.ini")),
        ..Default::default()
    };
    let texture_id = imgui::ImTexture::from(1);

    let image = ndarray::Array3::<f32>::zeros((10, 10, 10));
    let image = image.into_dyn();

    support::run(config, |ui, gl_ctx, textures| {
        ui.window(im_str!("Three")).build(|| {
            ui.image3d(&image, texture_id, textures, gl_ctx);
        });
        true
    })
    .unwrap();
}
