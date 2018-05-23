extern crate glium;
#[macro_use]
extern crate imgui;
extern crate imgui_glium_renderer;

extern crate aflak_cake as cake;
extern crate aflak_primitives as primitives;
extern crate node_editor;
extern crate ui_image1d;
extern crate ui_image2d;

use std::collections::HashMap;
use std::io::Cursor;

use node_editor::{ComputationState, ConstantEditor, NodeEditor};

use glium::backend::Facade;
use imgui::{ImString, Ui};
use imgui_glium_renderer::{AppConfig, AppContext};
use ui_image1d::UiImage1d;
use ui_image2d::UiImage2d;

const CLEAR_COLOR: [f32; 4] = [0.05, 0.05, 0.05, 1.0];

#[derive(Default)]
struct MyConstantEditor;

impl ConstantEditor<primitives::IOValue> for MyConstantEditor {
    fn editor(&self, ui: &Ui, constant: &mut primitives::IOValue) -> bool {
        use primitives::IOValue;
        match constant {
            &mut IOValue::Str(ref mut string) => {
                let mut out = ImString::with_capacity(1024);
                out.push_str(string);
                let changed = ui.input_text(im_str!("String value"), &mut out).build();
                *string = out.to_str().to_owned();
                changed
            }
            &mut IOValue::Integer(ref mut int) => {
                let mut out = *int as i32;
                let changed = ui.input_int(im_str!("Int value"), &mut out).build();
                *int = out as i64;
                changed
            }
            &mut IOValue::Float(ref mut float) => {
                ui.input_float(im_str!("Float value"), float).build()
            }
            &mut IOValue::Float2(ref mut floats) => {
                ui.input_float2(im_str!("2 floats value"), floats).build()
            }
            &mut IOValue::Float3(ref mut floats) => {
                ui.input_float3(im_str!("3 floats value"), floats).build()
            }
            _ => false,
        }
    }
}

fn main() {
    let transformations_ref = primitives::TRANSFORMATIONS.iter().collect::<Vec<_>>();
    let transformations = transformations_ref.as_slice();
    let import_data = Cursor::new(include_str!("output_image_2d.ron"));
    let mut node_editor =
        NodeEditor::from_export_buf(import_data, transformations, MyConstantEditor)
            .expect("Import failed");

    let mut app = AppContext::init(
        "Node editor example".to_owned(),
        AppConfig {
            clear_color: CLEAR_COLOR,
            ini_filename: Some(ImString::new("node_editor_example.ini")),
            ..Default::default()
        },
    ).unwrap();
    let gl_ctx = app.get_context().clone();
    let mut image1d_states = HashMap::new();
    let mut image2d_states = HashMap::new();
    app.run(|ui| {
        ui.window(im_str!("Node editor")).build(|| {
            node_editor.render(ui);
        });
        let outputs = node_editor.outputs();
        for output in outputs {
            let window_name = ImString::new(format!("{:?}", output));
            ui.window(&window_name).build(|| {
                let compute_state = node_editor.compute_output(&output);
                let compute_state = &*compute_state.lock().unwrap();
                match compute_state {
                    &ComputationState::NothingDone => {
                        ui.text("Initializing...");
                    }
                    &ComputationState::RunningFirstTime => {
                        ui.text("Computing...");
                    }
                    _ => match compute_state.result() {
                        Some(Err(ref e)) => {
                            ui.text(format!("{:?}", e));
                        }
                        Some(Ok(ref result)) => match result {
                            &primitives::IOValue::Str(ref string) => {
                                ui.text(format!("{:?}", string));
                            }
                            &primitives::IOValue::Integer(integer) => {
                                ui.text(format!("{:?}", integer));
                            }
                            &primitives::IOValue::Float(float) => {
                                ui.text(format!("{:?}", float));
                            }
                            &primitives::IOValue::Float2(floats) => {
                                ui.text(format!("{:?}", floats));
                            }
                            &primitives::IOValue::Float3(floats) => {
                                ui.text(format!("{:?}", floats));
                            }
                            &primitives::IOValue::Image1d(ref image) => {
                                let state = image1d_states
                                    .entry(window_name.clone())
                                    .or_insert_with(|| ui_image1d::State::default());
                                if let Err(e) = ui.image1d(image, state) {
                                    ui.text(format!("{:?}", e));
                                }
                            }
                            &primitives::IOValue::Image2d(ref image) => {
                                let state = image2d_states
                                    .entry(window_name.clone())
                                    .or_insert_with(|| ui_image2d::State::default());
                                if let Err(e) = ui.image2d(&gl_ctx, &window_name, image, state) {
                                    ui.text(format!("{:?}", e));
                                }
                            }
                            _ => {
                                ui.text("Unimplemented");
                            }
                        },
                        None => unreachable!(),
                    },
                }
            });
        }
        true
    }).unwrap();
}
