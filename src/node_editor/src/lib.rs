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

pub use constant_editor::ConstantEditor;
pub use layout::NodeEditorLayout;

pub struct NodeEditor<T: 'static, E: 'static> {
    layout: NodeEditorLayout<T, E>,
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
            if let Err(e) = event.execute(&mut self.layout.dst) {
                self.layout.error_stack.push(e);
            }
        }
    }

    /// Get all the outputs defined in the node editor.
    pub fn outputs(&self) -> Vec<cake::OutputId> {
        self.layout.outputs()
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
        NodeEditorLayout::from_export_buf(r).map(|layout| NodeEditor { layout })
    }
}

impl<T, E> Default for NodeEditor<T, E> {
    fn default() -> Self {
        NodeEditor {
            layout: Default::default(),
        }
    }
}
