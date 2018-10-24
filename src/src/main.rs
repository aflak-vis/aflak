//! # aflak - Advanced Framework for Learning Astrophysical Knowledge
//!
extern crate glium;
#[macro_use]
extern crate imgui;
extern crate imgui_glium_renderer;

extern crate aflak_cake as cake;
extern crate aflak_imgui_glium_support as support;
extern crate aflak_plot;
extern crate aflak_primitives as primitives;
extern crate imgui_file_explorer;
extern crate node_editor;

mod layout;
mod save_output;

use std::collections::HashMap;
use std::env;
use std::io::Cursor;

use node_editor::{ComputationState, ConstantEditor, NodeEditor};

use aflak_plot::{
    imshow::{self, UiImage2d},
    plot::{self, UiImage1d},
    InteractionId, ValueIter,
};
use imgui::{ImGuiCond, ImStr, ImString, ImTexture, Textures, Ui};
use imgui_file_explorer::UiFileExplorer;

use layout::LayoutEngine;

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
                        ret = ui.file_explorer(imgui_file_explorer::TOP_FOLDER, &["fits"]);
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
    env::set_var("WINIT_HIDPI_FACTOR", "1");

    let transformations_ref = primitives::TRANSFORMATIONS.iter().collect::<Vec<_>>();
    let transformations = transformations_ref.as_slice();
    let import_data = Cursor::new(include_str!("output_image_2d.ron"));
    let mut node_editor =
        NodeEditor::from_export_buf(import_data, transformations, MyConstantEditor)
            .expect("Import failed");

    let mut layout_engine = LayoutEngine::new();

    let mut image1d_states = HashMap::new();
    let mut image2d_states = HashMap::new();
    let mut editable_values = HashMap::new();
    let config = support::AppConfig {
        title: "aflak".to_owned(),
        clear_color: CLEAR_COLOR,
        ini_filename: Some(ImString::new("aflak.ini")),
        ..Default::default()
    };
    support::run(config, |ui, gl_ctx, textures| {
        ui.window(im_str!("Node editor"))
            .position(
                layout_engine.default_editor_window_position(),
                ImGuiCond::FirstUseEver,
            ).size(
                layout_engine.default_editor_window_size(),
                ImGuiCond::FirstUseEver,
            ).build(|| {
                node_editor.render(ui);
            });
        let outputs = node_editor.outputs();
        for output in outputs {
            let window_name = ImString::new(format!("Output #{}", output.id()));
            let (position, size) = layout_engine.default_output_window_position_size(&window_name);
            let window = ui
                .window(&window_name)
                .position(position, ImGuiCond::FirstUseEver)
                .size(size, ImGuiCond::FirstUseEver);
            window.build(|| {
                let compute_state = unsafe { node_editor.compute_output(&output) };
                let compute_state = &*compute_state.lock().unwrap();
                match compute_state {
                    &ComputationState::NothingDone => {
                        ui.text("Initializing...");
                    }
                    &ComputationState::RunningFirstTime => {
                        ui.text("Computing...");
                    }
                    _ => {
                        if let Some(result) = compute_state.result() {
                            match result {
                                Err(e) => ui.text(format!("{:?}", e)),
                                Ok(result) => output_window_computed_content(
                                    ui,
                                    result,
                                    &output,
                                    &window_name,
                                    &mut image1d_states,
                                    &mut image2d_states,
                                    &mut editable_values,
                                    &mut node_editor,
                                    gl_ctx,
                                    textures,
                                ),
                            };
                        } else {
                            // As per the present computation state,
                            // a result should be present. Else it's a bug.
                            unreachable!();
                        }
                    }
                }
            });
        }
        true
    }).unwrap();
}

fn output_window_computed_content<F>(
    ui: &Ui,
    result: &primitives::IOValue,
    output: &cake::OutputId,
    window_name: &ImStr,
    image1d_states: &mut HashMap<ImString, plot::State>,
    image2d_states: &mut HashMap<ImString, imshow::State>,
    editable_values: &mut HashMap<(cake::OutputId, InteractionId), cake::TransformIdx>,
    node_editor: &mut NodeEditor<primitives::IOValue, primitives::IOErr, MyConstantEditor>,
    gl_ctx: &F,
    textures: &mut Textures<glium::Texture2d>,
) where
    F: glium::backend::Facade,
{
    if ui.button(im_str!("Save data"), (0.0, 0.0)) {
        if let Err(e) = save_output::save(output, result) {
            eprintln!("Error on saving output: '{}'", e);
        } else {
            ui.open_popup(im_str!("FITS export completed!"));
        }
    }
    ui.popup_modal(im_str!("FITS export completed!")).build(|| {
        ui.text(format!(
            "File saved with success to '{}'.",
            save_output::file_name(output)
        ));
        if ui.button(im_str!("Close"), (0.0, 0.0)) {
            ui.close_current_popup();
        }
    });

    ui.new_line();

    match result {
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
                .entry(window_name.to_owned())
                .or_insert_with(|| plot::State::default());

            update_state_from_editor(
                &output,
                state.stored_values_mut(),
                editable_values,
                node_editor,
            );
            if let Err(e) = ui.image1d(image.scalar(), &format!("{}", image.array().unit()), state)
            {
                ui.text(format!("{:?}", e))
            }
            update_editor_from_state(&output, state.stored_values(), editable_values, node_editor);
        }
        &primitives::IOValue::Image2d(ref image) => {
            let state = image2d_states
                .entry(window_name.to_owned())
                .or_insert_with(|| imshow::State::default());

            update_state_from_editor(
                &output,
                state.stored_values_mut(),
                editable_values,
                node_editor,
            );
            let texture_id = ImTexture::from(hash_imstring(window_name));
            if let Err(e) = ui.image2d(
                gl_ctx,
                textures,
                texture_id,
                image.scalar(),
                &format!("{}", image.array().unit()),
                state,
            ) {
                ui.text(format!("{:?}", e));
            }
            update_editor_from_state(&output, state.stored_values(), editable_values, node_editor);
        }
        _ => {
            ui.text("Unimplemented");
        }
    }
}

fn update_state_from_editor(
    output: &cake::OutputId,
    interactions: aflak_plot::InteractionIterMut,
    store: &HashMap<(cake::OutputId, InteractionId), cake::TransformIdx>,
    node_editor: &NodeEditor<primitives::IOValue, primitives::IOErr, MyConstantEditor>,
) {
    for (id, interaction) in interactions {
        let value_id = (*output, *id);
        if store.contains_key(&value_id) {
            let t_idx = store.get(&value_id).unwrap();
            if let Some(value) = node_editor.constant_node_value(t_idx) {
                assert!(
                    value.len() == 1,
                    "Only constant nodes with exactly one value are supported",
                );
                let value = &value[0];
                if let Err(e) = match value {
                    primitives::IOValue::Integer(i) => interaction.set_value(*i),
                    primitives::IOValue::Float(f) => interaction.set_value(*f),
                    primitives::IOValue::Float2(f) => interaction.set_value(*f),
                    primitives::IOValue::Float3(f) => interaction.set_value(*f),
                    value => Err(format!("Cannot convert value '{:?}'", value)),
                } {
                    eprintln!("Could not update state from editor: {}", e);
                }
            } else {
                eprintln!("No constant node found for transform '{:?}'", t_idx);
            }
        } else {
            eprintln!("ValueID '{:?}' not found in store", value_id);
        }
    }
}

fn update_editor_from_state(
    output: &cake::OutputId,
    value_iter: ValueIter,
    store: &mut HashMap<(cake::OutputId, InteractionId), cake::TransformIdx>,
    node_editor: &mut NodeEditor<primitives::IOValue, primitives::IOErr, MyConstantEditor>,
) {
    for (id, value) in value_iter {
        use aflak_plot::Value;
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

/// Used to compute the ID of a texture
fn hash_imstring(string: &ImStr) -> usize {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;

    let mut h = DefaultHasher::new();
    h.write(string.to_str().as_bytes());
    h.finish() as usize
}
