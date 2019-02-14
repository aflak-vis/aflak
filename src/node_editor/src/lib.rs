//! A node editor library built on top of `aflak_cake` and `imgui`.
//!
//! For development you will want to check the
//! [NodeEditor](struct.NodeEditor.html) struct.
extern crate aflak_cake as cake;
#[macro_use]
extern crate imgui;
extern crate ron;

extern crate serde;
#[macro_use]
extern crate serde_derive;

mod compute;
mod constant_editor;
mod event;
mod export;
mod id_stack;
mod layout;
mod node_state;
mod scrolling;
mod vec2;

use std::{error, io};

use imgui::ImString;

pub use constant_editor::ConstantEditor;
pub use layout::NodeEditorLayout;

pub struct NodeEditor<T: 'static, E: 'static> {
    layout: NodeEditorLayout<T, E>,
    error_stack: Vec<Box<error::Error>>,
    success_stack: Vec<ImString>,
}

impl<T, E> NodeEditor<T, E>
where
    T: Clone + cake::VariantName + cake::ConvertibleVariants + Send + Sync,
    E: Send + Sync,
{
    /// Compute output's result asynchonously.
    pub fn compute_output(
        &mut self,
        id: cake::OutputId,
    ) -> Option<cake::compute::NodeResult<T, E>> {
        self.layout.compute_output(id)
    }
}

impl<T, E> NodeEditor<T, E>
where
    T: Clone + cake::VariantName,
{
    /// Add a constant node containing the value `t`.
    ///
    /// Return the ID if the new node.
    pub fn create_constant_node(&mut self, t: T) -> cake::TransformIdx {
        self.layout.create_constant_node(t)
    }
}

impl<T, E> NodeEditor<T, E>
where
    T: PartialEq + cake::VariantName,
{
    /// Update the constant value of constant node with given `id` with given
    /// value `val`.
    pub fn update_constant_node(&mut self, id: cake::TransformIdx, val: T) {
        self.layout.update_constant_node(id, val)
    }
}

impl<T, E> NodeEditor<T, E> {
    /// Get reference to value of contant node identified by `id`.
    pub fn constant_node_value(&self, id: cake::TransformIdx) -> Option<&T> {
        self.layout.constant_node_value(id)
    }
}

impl<T, E> NodeEditor<T, E>
where
    T: 'static
        + Clone
        + cake::EditableVariants
        + cake::NamedAlgorithms<E>
        + cake::VariantName
        + cake::DefaultFor
        + cake::ConvertibleVariants
        + serde::Serialize
        + for<'de> serde::Deserialize<'de>,
    E: 'static + error::Error,
{
    /// Draw the full node editor on the current window.
    pub fn render<ED>(
        &mut self,
        ui: &imgui::Ui,
        addable_nodes: &[&'static cake::Transform<T, E>],
        constant_editor: &ED,
    ) where
        ED: ConstantEditor<T>,
    {
        let events = self.layout.render(ui, addable_nodes, constant_editor);
        for event in events {
            self.apply_event(event);
        }

        self.render_error_popup(ui);
        self.render_success_popup(ui);
    }

    /// Get all the outputs defined in the node editor.
    pub fn outputs(&self) -> Vec<cake::OutputId> {
        self.layout.outputs()
    }

    fn render_error_popup(&mut self, ui: &imgui::Ui) {
        if !self.error_stack.is_empty() {
            ui.open_popup(im_str!("Error!"));
        }
        ui.popup_modal(im_str!("Error!")).build(|| {
            ui.with_text_wrap_pos(400.0, || {
                let e = &self.error_stack[self.error_stack.len() - 1];
                ui.text_wrapped(&ImString::new(format!("{}", e)));
            });
            if !ui.is_window_hovered() && ui.imgui().is_mouse_clicked(imgui::ImMouseButton::Left) {
                self.error_stack.pop();
                ui.close_current_popup();
            }
        });
    }
    fn render_success_popup(&mut self, ui: &imgui::Ui) {
        if self.error_stack.is_empty() && !self.success_stack.is_empty() {
            ui.open_popup(im_str!("Success!"));
        }
        ui.popup_modal(im_str!("Success!")).build(|| {
            {
                let msg = &self.success_stack[self.success_stack.len() - 1];
                ui.text(msg);
            }
            if !ui.is_window_hovered() && ui.imgui().is_mouse_clicked(imgui::ImMouseButton::Left) {
                self.success_stack.pop();
                ui.close_current_popup();
            }
        });
    }
}

impl<T, E> NodeEditor<T, E> {
    pub fn apply_event(&mut self, ev: event::RenderEvent<T, E>)
    where
        T: Clone
            + cake::ConvertibleVariants
            + cake::DefaultFor
            + cake::NamedAlgorithms<E>
            + cake::VariantName
            + serde::Serialize
            + for<'de> serde::Deserialize<'de>,
    {
        const EDITOR_EXPORT_FILE: &str = "editor_graph_export.ron";
        use event::RenderEvent::*;
        let errors = &mut self.error_stack;
        let successes = &mut self.success_stack;
        match ev {
            Connect(output, input_slot) => match input_slot {
                cake::InputSlot::Transform(input) => {
                    if let Err(e) = self.layout.dst.connect(output, input) {
                        eprintln!("{:?}", e);
                        errors.push(Box::new(e));
                    }
                }
                cake::InputSlot::Output(output_id) => {
                    self.layout.dst.update_output(output_id, output)
                }
            },
            AddTransform(t) => {
                self.layout.dst.add_transform(t);
            }
            CreateOutput => {
                self.layout.dst.create_output();
            }
            AddConstant(constant_type) => {
                let constant = cake::Transform::new_constant(T::default_for(constant_type));
                self.layout.dst.add_owned_transform(constant);
            }
            SetConstant(t_idx, val) => {
                if let Some(t) = self.layout.dst.get_transform_mut(t_idx) {
                    t.set_constant(*val);
                } else {
                    eprintln!("Transform {:?} was not found.", t_idx);
                }
            }
            WriteDefaultInput {
                t_idx,
                input_index,
                val,
            } => {
                if let Some(mut inputs) = self.layout.dst.get_default_inputs_mut(t_idx) {
                    inputs.write(input_index, *val);
                } else {
                    eprintln!("Transform {:?} was not found.", t_idx);
                }
            }
            RemoveNode(node_id) => {
                self.layout.dst.remove_node(&node_id);
            }
            Error(e) => errors.push(e),
            Success(msg) => successes.push(msg),
            Import => {
                if let Err(e) = self.layout.import_from_file(EDITOR_EXPORT_FILE) {
                    eprintln!("Error on import! {}", e);
                    errors.push(Box::new(e));
                }
            }
            Export => {
                if let Err(e) = self.layout.export_to_file(EDITOR_EXPORT_FILE) {
                    eprintln!("Error on export! {}", e);
                    errors.push(Box::new(e));
                } else {
                    successes.push(ImString::new(format!(
                        "Editor content was exported with success to '{}'!",
                        EDITOR_EXPORT_FILE
                    )));
                }
            }
        }
    }
}

impl<T, E> NodeEditor<T, E>
where
    T: 'static
        + Clone
        + cake::NamedAlgorithms<E>
        + cake::VariantName
        + cake::ConvertibleVariants
        + for<'de> serde::Deserialize<'de>,
    E: 'static,
{
    /// Deserialize a buffer in .ron format and make a node editor.
    pub fn from_export_buf<R>(r: R) -> Result<Self, export::ImportError>
    where
        R: io::Read,
    {
        NodeEditorLayout::from_export_buf(r).map(|layout| NodeEditor {
            layout,
            ..Default::default()
        })
    }
}

impl<T, E> Default for NodeEditor<T, E> {
    fn default() -> Self {
        NodeEditor {
            layout: Default::default(),
            error_stack: vec![],
            success_stack: vec![],
        }
    }
}
