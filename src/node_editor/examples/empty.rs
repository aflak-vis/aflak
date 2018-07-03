extern crate glium;
#[macro_use]
extern crate imgui;
extern crate imgui_glium_renderer;

extern crate aflak_cake as cake;
extern crate aflak_primitives as primitives;
extern crate imgui_file_explorer;
extern crate node_editor;
extern crate ui_image1d;
extern crate ui_image2d;

use std::collections::HashMap;
use std::io::Cursor;

use node_editor::{ComputationState, ConstantEditor, NodeEditor};

use glium::backend::Facade;
use imgui::{ImString, Ui};
use imgui_file_explorer::UiFileExplorer;
use imgui_glium_renderer::{AppConfig, AppContext};
use ui_image1d::UiImage1d;
use ui_image2d::{InteractionId, UiImage2d, ValueIter};

const CLEAR_COLOR: [f32; 4] = [0.05, 0.05, 0.05, 1.0];

#[derive(Default)]
struct MyConstantEditor;

impl ConstantEditor<primitives::IOValue> for MyConstantEditor {
    fn editor(&self, ui: &Ui, constant: &mut primitives::IOValue) -> bool {
        use primitives::IOValue;

        ui.push_id(constant as *const primitives::IOValue as i32);
        let changed = match constant {
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
            &mut IOValue::Path(ref mut file) => {
                ui.text(file.to_str().unwrap_or("Unrepresentable path"));
                let size = ui.get_item_rect_size();

                let mut ret = Ok(None);
                ui.child_frame(im_str!("edit"), (size.0.max(200.0), 150.0))
                    .scrollbar_horizontal(true)
                    .build(|| {
                        ret = ui.file_explorer("/home", &["fits"]);
                    });
                if let Ok(Some(new_file)) = ret {
                    if *file != new_file {
                        *file = new_file;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => false,
        };
        ui.pop_id();

        changed
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
    let mut editable_values = HashMap::new();
    app.run(|ui| {
        ui.window(im_str!("Node editor")).build(|| {
            node_editor.render(ui);
        });
        let outputs = node_editor.outputs();
        for output in outputs {
            let window_name = ImString::new(format!("{:?}", output));
            ui.window(&window_name).build(|| {
                let compute_state = unsafe { node_editor.compute_output(&output) };
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
                                    ui.text(format!("{:?}", e))
                                }
                                update_editor_from_state(
                                    &output,
                                    state.stored_values(),
                                    &mut editable_values,
                                    &mut node_editor,
                                );
                            }
                            &primitives::IOValue::Image2d(ref image) => {
                                let state = image2d_states
                                    .entry(window_name.clone())
                                    .or_insert_with(|| ui_image2d::State::default());
                                if let Err(e) = ui.image2d(&gl_ctx, &window_name, image, state) {
                                    ui.text(format!("{:?}", e));
                                }
                                update_editor_from_state(
                                    &output,
                                    state.stored_values(),
                                    &mut editable_values,
                                    &mut node_editor,
                                );
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

fn update_editor_from_state(
    output: &cake::OutputId,
    value_iter: ValueIter,
    store: &mut HashMap<(cake::OutputId, InteractionId), cake::TransformIdx>,
    node_editor: &mut NodeEditor<primitives::IOValue, primitives::IOErr, MyConstantEditor>,
) {
    for (id, value) in value_iter {
        use self::ui_image2d::Value;
        let val = match value {
            Value::Integer(i) => primitives::IOValue::Integer(i),
            Value::Float(f) => primitives::IOValue::Float(f),
            Value::Float2(f) => primitives::IOValue::Float2(f),
            Value::Float3(f) => primitives::IOValue::Float3(f),
        };
        let value_id = (*output, *id);
        if store.contains_key(&value_id) {
            let t_idx = store.get(&value_id).unwrap();
            node_editor.update_constant_node(t_idx, vec![val]);
        } else {
            let t_idx = node_editor.create_constant_node(val);
            store.insert(value_id, t_idx);
        }
    }
}
