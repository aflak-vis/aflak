use std::collections::HashMap;
use std::error;

use glium;
use imgui::{ImGuiCond, ImMouseButton, ImString, ImTexture, Ui};
use owning_ref::ArcRef;

use aflak_plot::{
    imshow::{self, Textures, UiImage2d},
    plot::{self, UiImage1d},
    AxisTransform, InteractionId, InteractionIterMut, ValueIter,
};
use cake::{OutputId, TransformIdx};
use node_editor::NodeEditor;
use primitives::{ndarray, IOErr, IOValue, SuccessOut, ROI};

use constant_editor::MyConstantEditor;
use layout::LayoutEngine;
use save_output;

pub type AflakNodeEditor = NodeEditor<'static, IOValue, IOErr, MyConstantEditor>;

pub struct Aflak {
    node_editor: AflakNodeEditor,
    layout_engine: LayoutEngine,
    output_windows: HashMap<OutputId, OutputWindow>,
    error_alerts: Vec<Box<error::Error>>,
}

#[derive(Default)]
struct OutputWindow {
    image1d_state: plot::State,
    image2d_state: imshow::State<ArcRef<IOValue, ndarray::ArrayD<f32>>>,
    editable_values: EditableValues,
    show_pixels: bool,
}

type EditableValues = HashMap<InteractionId, TransformIdx>;

impl Aflak {
    pub fn init(editor: AflakNodeEditor) -> Self {
        Self {
            node_editor: editor,
            layout_engine: LayoutEngine::new(),
            output_windows: HashMap::new(),
            error_alerts: vec![],
        }
    }

    pub fn node_editor(&mut self, ui: &Ui) {
        ui.window(im_str!("Node editor"))
            .position(
                self.layout_engine.default_editor_window_position(),
                ImGuiCond::FirstUseEver,
            )
            .size(
                self.layout_engine.default_editor_window_size(),
                ImGuiCond::FirstUseEver,
            )
            .build(|| {
                self.node_editor.render(ui);
            });
    }

    pub fn output_windows<F>(&mut self, ui: &Ui, gl_ctx: &F, textures: &mut Textures)
    where
        F: glium::backend::Facade,
    {
        let outputs = self.node_editor.outputs();
        for output in outputs {
            let output_window = self.output_windows.entry(output).or_default();
            output_window.draw(
                ui,
                output,
                &mut self.layout_engine,
                &mut self.node_editor,
                &mut self.error_alerts,
                gl_ctx,
                textures,
            );
        }
    }

    pub fn show_errors(&mut self, ui: &Ui) {
        if !self.error_alerts.is_empty() {
            ui.open_popup(im_str!("Error"));
        }
        ui.popup_modal(im_str!("Error")).build(|| {
            {
                let e = &self.error_alerts[self.error_alerts.len() - 1];
                ui.text(&ImString::new(format!("{}", e)));
            }
            if !ui.is_window_hovered() && ui.imgui().is_mouse_clicked(ImMouseButton::Left) {
                self.error_alerts.pop();
                ui.close_current_popup();
            }
        });
    }
}

impl OutputWindow {
    pub fn draw<F>(
        &mut self,
        ui: &Ui,
        output: OutputId,
        layout: &mut LayoutEngine,
        node_editor: &mut AflakNodeEditor,
        error_alerts: &mut Vec<Box<error::Error>>,
        gl_ctx: &F,
        textures: &mut Textures,
    ) where
        F: glium::backend::Facade,
    {
        let window_name = ImString::new(format!("Output #{}", output.id()));
        let display_size = ui.imgui().display_size();
        let (position, size) =
            layout.default_output_window_position_size(&window_name, display_size);
        let window = ui
            .window(&window_name)
            .position(position, ImGuiCond::FirstUseEver)
            .size(size, ImGuiCond::FirstUseEver)
            .menu_bar(true);
        window.build(|| {
            let compute_state = node_editor.compute_output(output);
            match compute_state {
                None => {
                    ui.text("Initializing...");
                }
                Some(Err(e)) => {
                    ui.text_wrapped(&ImString::new(format!("{}", e)));
                }
                Some(Ok(result)) => self.computed_content(
                    ui,
                    output,
                    result,
                    node_editor,
                    error_alerts,
                    gl_ctx,
                    textures,
                ),
            }
        });
    }

    fn computed_content<F>(
        &mut self,
        ui: &Ui,
        output: OutputId,
        result: SuccessOut,
        node_editor: &mut AflakNodeEditor,
        error_alerts: &mut Vec<Box<error::Error>>,
        gl_ctx: &F,
        textures: &mut Textures,
    ) where
        F: glium::backend::Facade,
    {
        let mut output_saved_success_popup = false;
        ui.menu_bar(|| {
            ui.menu(im_str!("File")).build(|| {
                if ui.menu_item(im_str!("Save")).build() {
                    if let Err(e) = save_output::save(output, &result) {
                        eprintln!("Error on saving output: '{}'", e);
                        error_alerts.push(Box::new(e));
                    } else {
                        output_saved_success_popup = true;
                    }
                }
                ui.menu_item(im_str!("Show pixels"))
                    .selected(&mut self.show_pixels)
                    .build();
            });
        });

        if output_saved_success_popup {
            ui.open_popup(im_str!("FITS export completed!"));
        }
        ui.popup_modal(im_str!("FITS export completed!")).build(|| {
            ui.text(format!(
                "File saved with success to '{}'.",
                save_output::file_name(&result, output)
            ));
            if ui.button(im_str!("Close"), (0.0, 0.0)) {
                ui.close_current_popup();
            }
        });

        let created_on = SuccessOut::created_on(&result);
        let value = SuccessOut::take(result);
        match &*value {
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
            IOValue::Bool(b) => {
                ui.text(format!("{:?}", b));
            }
            IOValue::Image(ref image) => {
                use primitives::ndarray::Dimension;
                match image.scalar().dim().ndim() {
                    1 => {
                        Self::update_state_from_editor(
                            self.image1d_state.stored_values_mut(),
                            &self.editable_values,
                            node_editor,
                        );
                        let unit = image.array().unit().repr();
                        let transform = if self.show_pixels {
                            None
                        } else {
                            match (image.cunits(), image.wcs()) {
                                (Some(units), Some(wcs)) => {
                                    Some(AxisTransform::new(units[0].repr(), move |t| {
                                        wcs.pix2world([t, 0.0, 0.0, 0.0])[0]
                                    }))
                                }
                                _ => None,
                            }
                        };
                        if let Err(e) =
                            ui.image1d(&image.scalar1(), &unit, transform, &mut self.image1d_state)
                        {
                            ui.text(format!("Error on drawing plot! {}", e))
                        }
                        Self::update_editor_from_state(
                            self.image1d_state.stored_values(),
                            &mut self.editable_values,
                            node_editor,
                        );
                    }
                    2 => {
                        Self::update_state_from_editor(
                            self.image2d_state.stored_values_mut(),
                            &self.editable_values,
                            node_editor,
                        );
                        let texture_id = ImTexture::from(hash_outputid(output));
                        let (x_transform, y_transform) = if self.show_pixels {
                            (None, None)
                        } else {
                            match (image.cunits(), image.wcs()) {
                                (Some(units), Some(wcs)) => (
                                    Some(AxisTransform::new(units[0].repr(), move |t| {
                                        wcs.pix2world([t, 0.0, 0.0, 0.0])[0]
                                    })),
                                    Some(AxisTransform::new(units[1].repr(), {
                                        let max_height =
                                            (image.scalar().dim().as_array_view().first().unwrap()
                                                - 1)
                                                as f32;
                                        move |t| wcs.pix2world([0.0, max_height - t, 0.0, 0.0])[1]
                                    })),
                                ),
                                _ => (None, None),
                            }
                        };
                        let unit = image.array().unit().repr();
                        let new_incoming_image = match self.image2d_state.image_created_on() {
                            Some(image_created_on) => created_on > image_created_on,
                            None => true,
                        };
                        if new_incoming_image {
                            let value_ref: ArcRef<_> = value.clone().into();
                            let image_ref = value_ref.map(|value| {
                                if let IOValue::Image(image) = value {
                                    image.scalar()
                                } else {
                                    unreachable!("Expect an Image")
                                }
                            });
                            if let Err(e) = self
                                .image2d_state
                                .set_image(image_ref, created_on, gl_ctx, texture_id, textures)
                            {
                                ui.text(format!("Error on creating image! {}", e));
                            }
                        }
                        if let Err(e) = ui.image2d(
                            gl_ctx,
                            textures,
                            texture_id,
                            unit,
                            x_transform,
                            y_transform,
                            &mut self.image2d_state,
                        ) {
                            ui.text(format!("Error on drawing image! {}", e));
                        }
                        Self::update_editor_from_state(
                            self.image2d_state.stored_values(),
                            &mut self.editable_values,
                            node_editor,
                        );
                    }
                    _ => {
                        ui.text(format!(
                            "Unimplemented for image of dimension {}",
                            image.scalar().ndim()
                        ));
                    }
                }
            }
            IOValue::Fits(ref fits) => {
                let mut has_hdus = false;
                for (i, hdu) in fits.iter().enumerate() {
                    use primitives::fitrs::HeaderValue::*;
                    use std::borrow::Cow;

                    has_hdus = true;

                    let tree_name = match hdu.value("EXTNAME") {
                        Some(CharacterString(extname)) => ImString::new(extname.as_str()),
                        _ => {
                            if i == 0 {
                                im_str!("Primary HDU").to_owned()
                            } else {
                                ImString::new(format!("Hdu #{}", i))
                            }
                        }
                    };

                    ui.push_id(i as i32);
                    ui.tree_node(&tree_name).build(|| {
                        for (key, value) in hdu {
                            ui.text(key);
                            if let Some(value) = value {
                                ui.same_line(150.0);
                                let value = match value {
                                    CharacterString(s) => Cow::Borrowed(s.as_str()),
                                    Logical(true) => Cow::Borrowed("True"),
                                    Logical(false) => Cow::Borrowed("False"),
                                    IntegerNumber(i) => Cow::Owned(format!("{}", i)),
                                    RealFloatingNumber(f) => Cow::Owned(format!("{:E}", f)),
                                    ComplexIntegerNumber(a, b) => {
                                        Cow::Owned(format!("{} + {}i", a, b))
                                    }
                                    ComplexFloatingNumber(a, b) => {
                                        Cow::Owned(format!("{:E} + {:E}i", a, b))
                                    }
                                };
                                ui.text(value);
                            }
                            ui.separator();
                        }
                    });
                    ui.pop_id();
                }
                if !has_hdus {
                    ui.text("Input Fits appears invalid. No HDU could be found.");
                }
            }
            _ => {
                ui.text("Unimplemented");
            }
        }
    }

    fn update_state_from_editor(
        interactions: InteractionIterMut,
        editable_values: &EditableValues,
        node_editor: &AflakNodeEditor,
    ) {
        for (id, interaction) in interactions {
            if editable_values.contains_key(id) {
                let t_idx = editable_values.get(id).unwrap();
                if let Some(value) = node_editor.constant_node_value(*t_idx) {
                    if let Err(e) = match value {
                        IOValue::Integer(i) => interaction.set_value(*i),
                        IOValue::Float(f) => interaction.set_value(*f),
                        IOValue::Float2(f) => interaction.set_value(*f),
                        IOValue::Float3(f) => interaction.set_value(*f),
                        IOValue::Roi(_) => Ok(()),
                        value => Err(format!("Cannot convert value '{:?}'", value)),
                    } {
                        eprintln!("Could not update state from editor: {}", e);
                    }
                } else {
                    eprintln!("No constant node found for transform '{:?}'", t_idx);
                }
            } else {
                eprintln!("'{:?}' not found in store", id);
            }
        }
    }

    fn update_editor_from_state(
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
                Value::FinedGrainedROI(pixels) => IOValue::Roi(ROI::PixelList(pixels)),
            };
            if store.contains_key(id) {
                let t_idx = *store.get(id).unwrap();
                node_editor.update_constant_node(t_idx, val);
            } else {
                let t_idx = node_editor.create_constant_node(val);
                store.insert(*id, t_idx);
            }
        }
    }
}

/// Used to compute the ID of a texture
fn hash_outputid(id: OutputId) -> usize {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut h = DefaultHasher::new();
    id.hash(&mut h);
    h.finish() as usize
}
