extern crate aflak_imgui_glium_support as support;
extern crate aflak_plot;
extern crate glium;
extern crate imgui;
extern crate ndarray;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use aflak_plot::{
    imshow::{self, UiImage2d},
    AxisTransform,
};

use imshow::cake::OutputId;
use imshow::node_editor::NodeEditor;

fn func(x: f32, y: f32, z: f32, c: f32, d: f32, r: f32) -> f32 {
    4.0 * c * c * ((x - r) * (x - r) + (z - r) * (z - r) + (x + r) * (x + r) + (z + r) * (z + r))
        - ((x - r) * (x - r) + y * y + (z - r) * (z - r) + c * c - d * d)
            * ((x - r) * (x - r) + y * y + (z - r) * (z - r) + c * c - d * d)
        - ((x + r) * (x + r) + y * y + (z + r) * (z + r) + c * c - d * d)
            * ((x + r) * (x + r) + y * y + (z + r) * (z + r) + c * c - d * d)
}

fn main() {
    let config = support::AppConfig {
        title: "Example".to_owned(),
        ini_filename: Some(PathBuf::from("sliced_metatorus.ini")),
        ..Default::default()
    };
    let mut state = imshow::State::default();
    let texture_id = imgui::TextureId::from(1);
    support::init(config).main_loop(move |ui, gl_ctx, textures| {
        if state.image_created_on().is_none() {
            let image_data = {
                const WIDTH: usize = 129;
                const HEIGHT: usize = 129;
                const c: f32 = 0.6;
                const d: f32 = 0.5;
                const R: f32 = 0.2;
                let mut image_data = Vec::with_capacity(WIDTH * HEIGHT);
                for j in 0..WIDTH {
                    for i in 0..HEIGHT {
                        let j = (j as f32) / 64.0 - 1.0;
                        let i = (i as f32) / 64.0 - 1.0;
                        let v = func(j as f32, 0.0, i as f32, c, d, R);
                        image_data.push(v);
                    }
                }
                ndarray::ArrayD::from_shape_vec(vec![WIDTH, HEIGHT], image_data).unwrap()
            };
            state
                .set_image(image_data, Instant::now(), gl_ctx, texture_id, textures)
                .unwrap();
        }
        imgui::Window::new(format!("Sliced Analytic Volume Function")).build(ui, || {
            ui.image2d(
                gl_ctx,
                textures,
                texture_id,
                "value",
                Some(&AxisTransform::new("X Axis", "", |x| x)),
                Some(&AxisTransform::new("Y Axis", "", |y| y)),
                &mut state,
                &mut None,
                &mut HashMap::new(),
                &mut None,
                OutputId::new(0),
                &NodeEditor::default(),
            )
            .expect("Image2d failed");
        });
        true
    });
}
