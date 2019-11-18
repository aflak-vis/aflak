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

    support::run(config, |ui, gl_ctx, textures| {
        ui.window(im_str!("Three")).build(|| {
            ui.image3d(&image, texture_id, textures, gl_ctx, &mut state);
        });
        true
    })
    .unwrap();
}
