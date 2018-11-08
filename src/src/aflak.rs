use std::collections::HashMap;

use glium;
use imgui::{ImGuiCond, ImStr, ImString, ImTexture, Ui};

use aflak_plot::{
    imshow::{self, Textures, UiImage2d},
    plot::{self, UiImage1d},
    AxisTransform, InteractionId, InteractionIterMut, ValueIter,
};
use cake::{OutputId, TransformIdx};
use node_editor::{ComputationState, NodeEditor};
use primitives::{IOErr, IOValue};

use constant_editor::MyConstantEditor;
use layout::LayoutEngine;
use save_output;

pub type AflakNodeEditor<'t> = NodeEditor<'t, IOValue, IOErr, MyConstantEditor>;

pub struct Aflak<'t> {
    node_editor: AflakNodeEditor<'t>,
    layout_engine: LayoutEngine,
    image1d_states: HashMap<ImString, plot::State>,
    image2d_states: HashMap<ImString, imshow::State>,
    editable_values: EditableValues,
}

type EditableValues = HashMap<(OutputId, InteractionId), TransformIdx>;

struct OutputWindow {
    output: OutputId,
    window_name: ImString,
}

impl<'t> Aflak<'t> {
    pub fn init(editor: AflakNodeEditor<'t>) -> Self {
        Self {
            node_editor: editor,
            layout_engine: LayoutEngine::new(),
            image1d_states: HashMap::new(),
            image2d_states: HashMap::new(),
            editable_values: HashMap::new(),
        }
    }

    pub fn node_editor(&mut self, ui: &Ui) {
        ui.window(im_str!("Node editor"))
            .position(
                self.layout_engine.default_editor_window_position(),
                ImGuiCond::FirstUseEver,
            ).size(
                self.layout_engine.default_editor_window_size(),
                ImGuiCond::FirstUseEver,
            ).build(|| {
                self.node_editor.render(ui);
            });
    }

    pub fn output_windows<F>(&mut self, ui: &Ui, gl_ctx: &F, textures: &mut Textures)
    where
        F: glium::backend::Facade,
    {
        let outputs = self.node_editor.outputs();
        for output in outputs {
            let mut output_window = self.output_window(output);
            output_window.draw(ui, self, gl_ctx, textures);
        }
    }

    fn output_window(&mut self, output: OutputId) -> OutputWindow {
        OutputWindow {
            output,
            window_name: ImString::new(format!("Output #{}", output.id())),
        }
    }
}

impl OutputWindow {
    fn draw<F>(&self, ui: &Ui, aflak: &mut Aflak, gl_ctx: &F, textures: &mut Textures)
    where
        F: glium::backend::Facade,
    {
        let (position, size) = aflak
            .layout_engine
            .default_output_window_position_size(&self.window_name);
        let window_name = self.window_name.to_owned();
        let window = ui
            .window(&window_name)
            .position(position, ImGuiCond::FirstUseEver)
            .size(size, ImGuiCond::FirstUseEver);
        window.build(|| {
            let compute_state = unsafe { aflak.node_editor.compute_output(&self.output) };
            let compute_state = &*compute_state.lock().unwrap();
            match *compute_state {
                ComputationState::NothingDone => {
                    ui.text("Initializing...");
                }
                ComputationState::RunningFirstTime => {
                    ui.text("Computing...");
                }
                _ => {
                    if let Some(result) = compute_state.result() {
                        match result {
                            Err(e) => ui.text_wrapped(&ImString::new(format!("{}", e))),
                            Ok(result) => {
                                self.computed_content(ui, aflak, result, gl_ctx, textures)
                            }
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

    fn computed_content<F>(
        &self,
        ui: &Ui,
        aflak: &mut Aflak,
        result: &IOValue,
        gl_ctx: &F,
        textures: &mut Textures,
    ) where
        F: glium::backend::Facade,
    {
        if ui.button(im_str!("Save data"), (0.0, 0.0)) {
            if let Err(e) = save_output::save(self.output, result) {
                eprintln!("Error on saving output: '{}'", e);
            } else {
                ui.open_popup(im_str!("FITS export completed!"));
            }
        }
        ui.popup_modal(im_str!("FITS export completed!")).build(|| {
            ui.text(format!(
                "File saved with success to '{}'.",
                save_output::file_name(self.output)
            ));
            if ui.button(im_str!("Close"), (0.0, 0.0)) {
                ui.close_current_popup();
            }
        });

        ui.new_line();

        match *result {
            IOValue::Str(ref string) => {
                ui.text(format!("{:?}", string));
            }
            IOValue::Integer(integer) => {
                ui.text(format!("{:?}", integer));
            }
            IOValue::Float(float) => {
                ui.text(format!("{:?}", float));
            }
            IOValue::Float2(floats) => {
                ui.text(format!("{:?}", floats));
            }
            IOValue::Float3(floats) => {
                ui.text(format!("{:?}", floats));
            }
            IOValue::Image1d(ref image) => {
                let state = aflak
                    .image1d_states
                    .entry(self.window_name.to_owned())
                    .or_insert_with(plot::State::default);

                self.update_state_from_editor(
                    state.stored_values_mut(),
                    &aflak.editable_values,
                    &aflak.node_editor,
                );
                let unit = image.array().unit().repr();
                let transform = match (image.cunit(), image.wcs()) {
                    (Some(unit), Some(wcs)) => Some(AxisTransform::new(unit.repr(), move |t| {
                        wcs.pix2world([t, 0.0, 0.0])[0]
                    })),
                    _ => None,
                };
                if let Err(e) = ui.image1d(image.scalar(), &unit, transform, state) {
                    ui.text(format!("Error on drawing plot! {}", e))
                }
                self.update_editor_from_state(
                    state.stored_values(),
                    &mut aflak.editable_values,
                    &mut aflak.node_editor,
                );
            }
            IOValue::Image2d(ref image) => {
                let state = aflak
                    .image2d_states
                    .entry(self.window_name.to_owned())
                    .or_insert_with(imshow::State::default);

                self.update_state_from_editor(
                    state.stored_values_mut(),
                    &aflak.editable_values,
                    &aflak.node_editor,
                );
                let texture_id = ImTexture::from(hash_imstring(&self.window_name));
                let (x_transform, y_transform) = match (image.cunits(), image.wcs()) {
                    (Some(units), Some(wcs)) => (
                        Some(AxisTransform::new(units[0].repr(), move |t| {
                            wcs.pix2world([t, 0.0, 0.0])[0]
                        })),
                        Some(AxisTransform::new(units[1].repr(), {
                            let max_height = (image.scalar().dim().0 - 1) as f32;
                            move |t| wcs.pix2world([0.0, max_height - t, 0.0])[1]
                        })),
                    ),
                    _ => (None, None),
                };
                if let Err(e) = ui.image2d(
                    gl_ctx,
                    textures,
                    texture_id,
                    image.scalar(),
                    image.array().unit().repr(),
                    x_transform,
                    y_transform,
                    state,
                ) {
                    ui.text(format!("Error on drawing image! {}", e));
                }
                self.update_editor_from_state(
                    state.stored_values(),
                    &mut aflak.editable_values,
                    &mut aflak.node_editor,
                );
            }
            _ => {
                ui.text("Unimplemented");
            }
        }
    }

    fn update_state_from_editor(
        &self,
        interactions: InteractionIterMut,
        editable_values: &EditableValues,
        node_editor: &AflakNodeEditor,
    ) {
        for (id, interaction) in interactions {
            let value_id = (self.output, *id);
            if editable_values.contains_key(&value_id) {
                let t_idx = editable_values.get(&value_id).unwrap();
                if let Some(value) = node_editor.constant_node_value(t_idx) {
                    assert!(
                        value.len() == 1,
                        "Only constant nodes with exactly one value are supported",
                    );
                    let value = &value[0];
                    if let Err(e) = match value {
                        IOValue::Integer(i) => interaction.set_value(*i),
                        IOValue::Float(f) => interaction.set_value(*f),
                        IOValue::Float2(f) => interaction.set_value(*f),
                        IOValue::Float3(f) => interaction.set_value(*f),
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
        &self,
        value_iter: ValueIter,
        store: &mut EditableValues,
        node_editor: &mut AflakNodeEditor,
    ) {
        for (id, value) in value_iter {
            use aflak_plot::Value;
            let val = match value {
                Value::Integer(i) => IOValue::Integer(i),
                Value::Float(f) => IOValue::Float(f),
                Value::Float2(f) => IOValue::Float2(f),
                Value::Float3(f) => IOValue::Float3(f),
            };
            let value_id = (self.output, *id);
            if store.contains_key(&value_id) {
                let t_idx = store.get(&value_id).unwrap();
                node_editor.update_constant_node(t_idx, vec![val]);
            } else {
                let t_idx = node_editor.create_constant_node(val);
                store.insert(value_id, t_idx);
            }
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
