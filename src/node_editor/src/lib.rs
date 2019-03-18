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

mod constant_editor;
mod event;
mod export;
mod id_stack;
mod layout;
mod node_state;
mod scrolling;
mod vec2;

use std::{collections, error, fs, io, path};

use cake::Future;
use imgui::ImString;

pub use constant_editor::ConstantEditor;
use event::ApplyRenderEvent;
use layout::NodeEditorLayout;

/// The node editor instance.
pub struct NodeEditor<T: 'static, E: 'static> {
    dst: cake::DST<'static, T, E>,
    output_results: collections::BTreeMap<cake::OutputId, ComputationState<T, E>>,
    cache: cake::Cache<T, cake::compute::ComputeError<E>>,
    macros: cake::macros::MacroManager<'static, T, E>,
    layout: NodeEditorLayout<T, E>,
    error_stack: Vec<Box<error::Error>>,
    success_stack: Vec<ImString>,

    nodes_edit: Vec<InnerNodeEditor<T, E>>,
}

struct InnerNodeEditor<T: 'static, E: 'static> {
    handle: cake::macros::MacroHandle<'static, T, E>,
    layout: NodeEditorLayout<T, E>,
}

impl<T, E> InnerNodeEditor<T, E> {
    fn new(handle: &cake::macros::MacroHandle<'static, T, E>) -> Self {
        Self {
            handle: handle.clone(),
            layout: Default::default(),
        }
    }
}

struct ComputationState<T, E> {
    previous_result: Option<cake::compute::NodeResult<T, E>>,
    task: cake::Task<cake::compute::SuccessOut<T>, cake::compute::ErrorOut<E>>,
    counter: u8,
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
        let dst = &mut self.dst;
        let cache = &mut self.cache;
        let state = self
            .output_results
            .entry(id)
            .or_insert_with(|| ComputationState {
                previous_result: None,
                task: dst.compute(id, cache),
                counter: 1,
            });

        const WRAP: u8 = 5;
        if state.counter % WRAP == 0 {
            match state.task.poll() {
                Ok(cake::Async::Ready(t)) => {
                    state.previous_result = Some(Ok(t));
                    state.task = dst.compute(id, cache);
                }
                Ok(cake::Async::NotReady) => (),
                Err(e) => {
                    state.previous_result = Some(Err(e));
                    state.task = dst.compute(id, cache);
                }
            };
            dst.update_defaults_from_cache(cache);
        }
        if state.counter == WRAP - 1 {
            state.counter = 0;
        } else {
            state.counter += 1;
        }
        state.previous_result.clone()
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
        self.dst
            .add_owned_transform(cake::Transform::new_constant(t))
    }
}

impl<T, E> NodeEditor<T, E>
where
    T: PartialEq + cake::VariantName,
{
    /// Update the constant value of constant node with given `id` with given
    /// value `val`.
    pub fn update_constant_node(&mut self, id: cake::TransformIdx, val: T) {
        if let Some(t) = self.dst.get_transform_mut(id) {
            let mut new_value = false;
            if let cake::Algorithm::Constant(ref constant) = t.algorithm() {
                new_value = *constant != val;
            }
            if new_value {
                t.set_constant(val);
            }
        }
    }
}

impl<T, E> NodeEditor<T, E> {
    /// Get reference to value of contant node identified by `id`.
    pub fn constant_node_value(&self, id: cake::TransformIdx) -> Option<&T> {
        self.dst.get_transform(id).and_then(|t| {
            if let cake::Algorithm::Constant(ref constant) = t.algorithm() {
                Some(constant)
            } else {
                None
            }
        })
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
        let events = self
            .layout
            .render(ui, &self.dst, addable_nodes, constant_editor);
        for event in events {
            self.apply_event(event);
        }

        self.render_error_popup(ui);
        self.render_success_popup(ui);
    }

    /// Get all the outputs defined in the node editor.
    pub fn outputs(&self) -> Vec<cake::OutputId> {
        self.dst
            .outputs_iter()
            .filter(|(_, some_output)| some_output.is_some())
            .map(|(id, _)| *id)
            .collect()
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

    pub fn inner_editors_render<ED>(
        &mut self,
        ui: &imgui::Ui,
        addable_nodes: &[&'static cake::Transform<T, E>],
        constant_editor: &ED,
    ) where
        ED: ConstantEditor<T>,
    {
        for (i, node_edit) in self.nodes_edit.iter_mut().enumerate() {
            ui.window(&imgui::ImString::new(format!(
                "Macro editor: '{}'###{}",
                node_edit.handle.name(),
                i,
            )))
            .build(|| {
                let events = {
                    let lock = node_edit.handle.read();
                    let dst = lock.dst();
                    node_edit
                        .layout
                        .render(ui, dst, addable_nodes, constant_editor)
                };
                for event in events {
                    node_edit.apply_event(event);
                }
            })
        }
    }
}

const EDITOR_EXPORT_FILE: &str = "editor_graph_export.ron";

impl<T, E> ApplyRenderEvent<T, E> for NodeEditor<T, E>
where
    T: Clone
        + cake::ConvertibleVariants
        + cake::DefaultFor
        + cake::NamedAlgorithms<E>
        + cake::VariantName
        + serde::Serialize
        + for<'de> serde::Deserialize<'de>,
{
    fn connect(&mut self, output: cake::Output, input_slot: cake::InputSlot) {
        match input_slot {
            cake::InputSlot::Transform(input) => {
                if let Err(e) = self.dst.connect(output, input) {
                    eprintln!("{:?}", e);
                    self.error_stack.push(Box::new(e));
                }
            }
            cake::InputSlot::Output(output_id) => self.dst.update_output(output_id, output),
        }
    }
    fn add_transform(&mut self, t: &'static cake::Transform<'static, T, E>) {
        self.dst.add_transform(t);
    }
    fn create_output(&mut self) {
        self.dst.create_output();
    }
    fn add_constant(&mut self, constant_type: &'static str) {
        let constant = cake::Transform::new_constant(T::default_for(constant_type));
        self.dst.add_owned_transform(constant);
    }
    fn set_constant(&mut self, t_idx: cake::TransformIdx, c: Box<T>) {
        if let Some(t) = self.dst.get_transform_mut(t_idx) {
            t.set_constant(*c);
        } else {
            eprintln!("Transform {:?} was not found.", t_idx);
        }
    }
    fn write_default_input(&mut self, t_idx: cake::TransformIdx, input_index: usize, val: Box<T>) {
        if let Some(mut inputs) = self.dst.get_default_inputs_mut(t_idx) {
            inputs.write(input_index, *val);
        } else {
            eprintln!("Transform {:?} was not found.", t_idx);
        }
    }
    fn remove_node(&mut self, node_id: cake::NodeId) {
        self.dst.remove_node(&node_id);
    }
    fn import(&mut self) {
        if let Err(e) = self.import_from_file(EDITOR_EXPORT_FILE) {
            eprintln!("Error on import! {}", e);
            self.error_stack.push(Box::new(e));
        }
    }
    fn export(&mut self) {
        if let Err(e) = self.export_to_file(EDITOR_EXPORT_FILE) {
            eprintln!("Error on export! {}", e);
            self.error_stack.push(Box::new(e));
        } else {
            self.success_stack.push(ImString::new(format!(
                "Editor content was exported with success to '{}'!",
                EDITOR_EXPORT_FILE
            )));
        }
    }
    fn add_new_macro(&mut self) {
        self.dst.add_owned_transform(cake::Transform::from_macro(
            self.macros.create_macro().clone(),
        ));
    }
    fn edit_node(&mut self, node_id: cake::NodeId) {
        if let cake::NodeId::Transform(t_idx) = node_id {
            if let Some(t) = self.dst.get_transform(t_idx) {
                if let cake::Algorithm::Macro { handle } = t.algorithm() {
                    self.nodes_edit.push(InnerNodeEditor::new(handle))
                }
            }
        }
    }
}

impl<T, E> ApplyRenderEvent<T, E> for InnerNodeEditor<T, E>
where
    T: Clone + cake::ConvertibleVariants + cake::DefaultFor,
{
    fn connect(&mut self, output: cake::Output, input_slot: cake::InputSlot) {
        let mut lock = self.handle.write();
        let dst = lock.dst_mut();
        match input_slot {
            cake::InputSlot::Transform(input) => {
                if let Err(e) = dst.connect(output, input) {
                    eprintln!("Cannot connect in macro: {:?}", e);
                    // TODO: Error stack
                }
            }
            cake::InputSlot::Output(output_id) => dst.update_output(output_id, output),
        }
    }
    fn add_transform(&mut self, t: &'static cake::Transform<'static, T, E>) {
        self.handle.write().dst_mut().add_transform(t);
    }
    fn create_output(&mut self) {
        self.handle.write().dst_mut().create_output();
    }
    fn add_constant(&mut self, constant_type: &'static str) {
        let constant = cake::Transform::new_constant(T::default_for(constant_type));
        self.handle.write().dst_mut().add_owned_transform(constant);
    }
    fn set_constant(&mut self, t_idx: cake::TransformIdx, c: Box<T>) {
        let mut lock = self.handle.write();
        let dst = lock.dst_mut();
        if let Some(t) = dst.get_transform_mut(t_idx) {
            t.set_constant(*c);
        } else {
            eprintln!("Transform {:?} was not found in macro.", t_idx,);
        }
    }
    fn write_default_input(&mut self, t_idx: cake::TransformIdx, input_index: usize, val: Box<T>) {
        let mut lock = self.handle.write();
        let dst = lock.dst_mut();
        if let Some(mut inputs) = dst.get_default_inputs_mut(t_idx) {
            inputs.write(input_index, *val);
        } else {
            eprintln!("Transform {:?} was not found.", t_idx);
        }
    }
    fn remove_node(&mut self, node_id: cake::NodeId) {
        self.handle.write().dst_mut().remove_node(&node_id);
    }
    fn import(&mut self) {
        eprintln!("Import unsupported in MacroEditor!");
    }
    fn export(&mut self) {
        eprintln!("Export unsupported in MacroEditor!");
    }
    fn add_new_macro(&mut self) {
        eprintln!("Nested macro not supported!");
    }
    fn edit_node(&mut self, node_id: cake::NodeId) {
        eprintln!("Unimplemented event: EditNode({:?})", node_id)
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
        let mut editor = Self::default();
        editor.import_from_buf(r)?;
        Ok(editor)
    }
}

#[derive(Serialize)]
struct SerialEditor<'e, T: 'e> {
    macros: cake::macros::SerdeMacroManager<T>,
    dst: cake::SerialDST<'e, T>,
    node_states: Vec<(&'e cake::NodeId, &'e node_state::NodeState)>,
    scrolling: vec2::Vec2,
}

impl<'e, T> SerialEditor<'e, T>
where
    T: Clone + cake::VariantName,
{
    fn new<E>(editor: &'e NodeEditor<T, E>) -> Self {
        Self {
            dst: cake::SerialDST::new(&editor.dst),
            node_states: editor.layout.node_states().iter().collect(),
            scrolling: editor.layout.scrolling().get_current(),
            macros: editor.macros.to_serializable(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(bound(deserialize = "T: serde::Deserialize<'de>"))]
struct DeserEditor<T> {
    macros: cake::macros::SerdeMacroManager<T>,
    dst: cake::DeserDST<T>,
    node_states: Vec<(cake::NodeId, node_state::NodeState)>,
    scrolling: vec2::Vec2,
}

impl<T, E> NodeEditor<T, E>
where
    T: Clone
        + cake::NamedAlgorithms<E>
        + cake::VariantName
        + cake::ConvertibleVariants
        + for<'de> serde::Deserialize<'de>,
{
    fn import_from_file<P: AsRef<path::Path>>(
        &mut self,
        file_path: P,
    ) -> Result<(), export::ImportError> {
        let f = fs::File::open(file_path)?;
        self.import_from_buf(f)
    }

    fn import_from_buf<R: io::Read>(&mut self, r: R) -> Result<(), export::ImportError> {
        let deserialized: DeserEditor<T> = ron::de::from_reader(r)?;
        self.macros.from_deserializable(deserialized.macros)?;
        self.dst = deserialized.dst.into_dst(&self.macros)?;

        let node_states = {
            let mut node_states = node_state::NodeStates::new();
            for (node_id, state) in deserialized.node_states {
                node_states.insert(node_id, state);
            }
            node_states
        };
        let scrolling = scrolling::Scrolling::new(deserialized.scrolling);
        self.layout.import(node_states, scrolling);

        // Reset cache
        self.output_results = collections::BTreeMap::new();
        self.cache = cake::Cache::new();

        // Close macro editing windows
        self.nodes_edit = vec![];

        Ok(())
    }
}

impl<T, E> NodeEditor<T, E>
where
    T: Clone + serde::Serialize + cake::VariantName,
{
    /// Serialize node editor to writer as .ron format.
    fn export_to_buf<W: io::Write>(&self, w: &mut W) -> Result<(), export::ExportError> {
        let serializable = SerialEditor::new(self);
        let serialized = ron::ser::to_string_pretty(&serializable, Default::default())?;
        w.write_all(serialized.as_bytes())?;
        w.flush()?;
        Ok(())
    }

    /// Serialize node editor to .ron file.
    fn export_to_file<P: AsRef<path::Path>>(
        &self,
        file_path: P,
    ) -> Result<(), export::ExportError> {
        let mut f = fs::File::create(file_path)?;
        self.export_to_buf(&mut f)
    }
}

impl<T, E> Default for NodeEditor<T, E> {
    fn default() -> Self {
        NodeEditor {
            dst: Default::default(),
            output_results: collections::BTreeMap::new(),
            cache: cake::Cache::new(),
            macros: cake::macros::MacroManager::new(),
            layout: Default::default(),
            error_stack: vec![],
            success_stack: vec![],
            nodes_edit: vec![],
        }
    }
}
