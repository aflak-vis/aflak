//! A node editor library built on top of `aflak_cake` and `imgui`.
//!
//! For development you will want to check the
//! [NodeEditor](struct.NodeEditor.html) struct.
extern crate aflak_cake as cake;
extern crate imgui;
extern crate imgui_file_explorer;
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

use std::{collections, error, fmt, fs, io, path};

use crate::cake::{Future, TransformIdx};
use imgui::ImString;
use imgui_file_explorer::UiFileExplorer;

pub use crate::constant_editor::ConstantEditor;
use crate::event::ApplyRenderEvent;
use crate::layout::NodeEditorLayout;

/// The node editor instance.
pub struct NodeEditor<T: 'static, E: 'static> {
    pub dst: cake::DST<'static, T, E>,
    output_results: collections::BTreeMap<cake::OutputId, ComputationState<T, E>>,
    cache: cake::Cache<T, cake::compute::ComputeError<E>>,
    pub macros: cake::macros::MacroManager<'static, T, E>,
    layout: NodeEditorLayout<T, E>,
    error_stack: Vec<Box<dyn error::Error>>,
    success_stack: Vec<ImString>,
    nodes_edit: Vec<InnerNodeEditor<T, E>>,
    import_macro: Option<cake::macros::MacroHandle<'static, T, E>>,
    pub valid_history: Vec<event::ProvenanceEvent<T, E>>,
    redo_stack: Vec<event::ProvenanceEvent<T, E>>,
}

struct InnerNodeEditor<T: 'static, E: 'static> {
    handle: cake::macros::MacroHandle<'static, T, E>,
    layout: NodeEditorLayout<T, E>,
    opened: bool,
    focus: bool,

    error_stack: Vec<InnerEditorError>,
    success_stack: Vec<ImString>,
    pub valid_history: Vec<event::ProvenanceEvent<T, E>>,
    redo_stack: Vec<event::ProvenanceEvent<T, E>>,
}

impl<T, E> InnerNodeEditor<T, E> {
    fn new(handle: cake::macros::MacroHandle<'static, T, E>) -> Self {
        Self {
            handle,
            layout: Default::default(),
            opened: true,
            focus: true,
            error_stack: vec![],
            success_stack: vec![],
            valid_history: vec![event::ProvenanceEvent::Import(
                None,
                None,
                cake::DST::new(),
                Ok(()),
            )],
            redo_stack: vec![],
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
            .add_owned_transform(cake::Transform::new_constant(t), None)
    }
}

impl<T, E> NodeEditor<T, E>
where
    T: PartialEq + cake::VariantName + std::clone::Clone,
{
    /// Update the constant value of constant node with given `id` with given
    /// value `val`.
    pub fn update_constant_node(&mut self, id: cake::TransformIdx, val: T) {
        if let Some(macro_id) = id.macro_id() {
            if let Some(macro_handle) = self.macros.get_macro(macro_id) {
                if let Some(t) = macro_handle.write().dst_mut().get_transform_mut(id) {
                    let mut new_value = false;
                    if let cake::Algorithm::Constant(ref constant) = t.algorithm() {
                        new_value = *constant != val;
                    }
                    if new_value {
                        t.set_constant(val);
                    }
                } else {
                    eprintln!("not found macro transform from {:?}", id);
                }
            } else {
                eprintln!("not found macro handle");
            }
        } else {
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
        attaching: &mut Option<(cake::OutputId, TransformIdx, usize)>,
    ) where
        ED: ConstantEditor<T>,
    {
        let events = self.layout.render(
            ui,
            &self.dst,
            addable_nodes,
            &self.macros,
            constant_editor,
            attaching,
        );
        for event in events {
            use event::ProvenanceEvent;
            use event::RenderEvent;

            let push_event = RenderEvent::new(&event);
            self.apply_event(event);
            let pushed = self.valid_history.pop().unwrap();
            if self.valid_history.is_empty() {
                match push_event {
                    RenderEvent::Undo => {
                        self.valid_history.push(pushed);
                        println!("Cannot Undo");
                        continue;
                    }
                    _ => {}
                }
            }
            if self.redo_stack.is_empty() {
                match push_event {
                    RenderEvent::Redo => {
                        self.valid_history.push(pushed);
                        println!("Cannot Redo");
                        continue;
                    }
                    _ => {}
                }
            }
            match (push_event, &pushed) {
                (RenderEvent::Connect(_, _), ProvenanceEvent::Connect(_, _, Ok(())))
                | (RenderEvent::Disconnect(_, _), ProvenanceEvent::Disconnect(_, _, Ok(())))
                | (RenderEvent::AddTransform(_), ProvenanceEvent::AddTransform(_, _))
                | (RenderEvent::CreateOutput, ProvenanceEvent::CreateOutput(_))
                | (RenderEvent::AddConstant(_), ProvenanceEvent::AddConstant(_, _))
                | (RenderEvent::SetConstant(_, _), ProvenanceEvent::SetConstant(_, _, _, Ok(())))
                | (
                    RenderEvent::WriteDefaultInput { .. },
                    ProvenanceEvent::WriteDefaultInput(_, _, _, _, Ok(())),
                )
                | (RenderEvent::RemoveNode(_), ProvenanceEvent::RemoveNode(_, _, _, _, _, _))
                | (RenderEvent::Import, ProvenanceEvent::Import(_, _, _, Ok(())))
                | (RenderEvent::Export, ProvenanceEvent::Export(_, _, Ok(())))
                | (RenderEvent::AddNewMacro, ProvenanceEvent::AddNewMacro(_))
                | (RenderEvent::AddMacro(_), ProvenanceEvent::AddMacro(_, _))
                | (RenderEvent::EditNode(_), ProvenanceEvent::EditNode(_))
                | (
                    RenderEvent::ChangeOutputName(_, _),
                    ProvenanceEvent::ChangeOutputName(_, _, _),
                ) => {
                    self.redo_stack.clear();
                    self.valid_history.push(pushed);
                }
                //Undo by looking at the previous operation, the opposite event is executed.
                (RenderEvent::Undo, ProvenanceEvent::Connect(o, i, Ok(()))) => {
                    let alternative_event = RenderEvent::<T, E>::Disconnect(*o, *i);
                    self.apply_event(alternative_event);
                    self.valid_history.pop();
                    self.redo_stack.push(pushed);
                }
                (RenderEvent::Undo, ProvenanceEvent::Disconnect(o, i, Ok(()))) => {
                    let alternative_event = RenderEvent::<T, E>::Connect(*o, *i);
                    self.apply_event(alternative_event);
                    self.valid_history.pop();
                    self.redo_stack.push(pushed);
                }
                (RenderEvent::Undo, ProvenanceEvent::AddTransform(_, t_idx))
                | (RenderEvent::Undo, ProvenanceEvent::AddConstant(_, t_idx)) => {
                    let node = cake::NodeId::Transform(*t_idx);
                    let alternative_event = RenderEvent::<T, E>::RemoveNode(node);
                    self.apply_event(alternative_event);
                    self.valid_history.pop();
                    self.redo_stack.push(pushed);
                }
                (RenderEvent::Undo, ProvenanceEvent::CreateOutput(output_id)) => {
                    let node = cake::NodeId::Output(*output_id);
                    let alternative_event = RenderEvent::<T, E>::RemoveNode(node);
                    self.apply_event(alternative_event);
                    self.valid_history.pop();
                    self.redo_stack.push(pushed);
                }
                (RenderEvent::Undo, ProvenanceEvent::SetConstant(t_idx, before, _, Ok(()))) => {
                    let alternative_event =
                        RenderEvent::<T, E>::SetConstant(*t_idx, Box::new(before.clone().unwrap()));
                    self.apply_event(alternative_event);
                    self.valid_history.pop();
                    self.redo_stack.push(pushed);
                }
                (
                    RenderEvent::Undo,
                    ProvenanceEvent::WriteDefaultInput(t_idx, input_index, before, _, Ok(())),
                ) => {
                    let alternative_event = RenderEvent::<T, E>::WriteDefaultInput {
                        t_idx: *t_idx,
                        input_index: *input_index,
                        val: Box::new(before.clone().unwrap()),
                    };
                    self.apply_event(alternative_event);
                    self.valid_history.pop();
                    self.redo_stack.push(pushed);
                }
                (
                    RenderEvent::Undo,
                    ProvenanceEvent::RemoveNode(cake::NodeId::Transform(_), t, _, d_i, i_c, o_c),
                ) => {
                    self.apply_event(RenderEvent::<T, E>::AddOwnedTransform(
                        t.clone(),
                        d_i.clone(),
                        i_c.clone(),
                        o_c.clone(),
                    ));
                    self.valid_history.pop();
                    self.redo_stack.push(pushed);
                }
                (
                    RenderEvent::Undo,
                    ProvenanceEvent::RemoveNode(
                        cake::NodeId::Output(output_id),
                        _,
                        name,
                        _,
                        i_c,
                        _,
                    ),
                ) => {
                    if i_c.len() == 1 {
                        if let Some(output) = i_c.get(0).unwrap() {
                            self.dst.attach_output_with_id_name(
                                *output,
                                *output_id,
                                name.clone().unwrap(),
                            );
                        } else {
                            self.dst
                                .create_output_with_id_name(*output_id, name.clone().unwrap());
                        }
                    }
                    self.redo_stack.push(pushed);
                }
                (RenderEvent::Undo, ProvenanceEvent::Import(_, Some(before_dst), _, Ok(()))) => {
                    self.dst = before_dst.clone();
                    self.redo_stack.push(pushed);
                }
                (RenderEvent::Undo, ProvenanceEvent::Export(_, _, _)) => {
                    println!("Export undo skipped.");
                    self.redo_stack.push(pushed);
                }
                (RenderEvent::Undo, ProvenanceEvent::AddNewMacro(t_idx)) => {
                    let node = cake::NodeId::Transform(*t_idx);
                    let alternative_event = RenderEvent::<T, E>::RemoveNode(node);
                    self.apply_event(alternative_event);
                    self.valid_history.pop();
                    self.redo_stack.push(pushed);
                }
                (RenderEvent::Undo, ProvenanceEvent::AddMacro(_, t_idx)) => {
                    let node = cake::NodeId::Transform(*t_idx);
                    let alternative_event = RenderEvent::<T, E>::RemoveNode(node);
                    self.apply_event(alternative_event);
                    self.valid_history.pop();
                    self.redo_stack.push(pushed);
                }
                (RenderEvent::Undo, ProvenanceEvent::EditNode(_)) => {
                    println!("EditNode skipped.");
                    self.redo_stack.push(pushed);
                }
                (RenderEvent::Undo, ProvenanceEvent::ChangeOutputName(node_id, before_name, _)) => {
                    let alternative_event =
                        RenderEvent::<T, E>::ChangeOutputName(*node_id, before_name.clone());
                    self.apply_event(alternative_event);
                    self.valid_history.pop();
                    self.redo_stack.push(pushed);
                }
                (RenderEvent::Redo, _) => {
                    if let Some(redo_event) = self.redo_stack.pop() {
                        self.valid_history.push(pushed);
                        let redo_event = RenderEvent::<T, E>::from(redo_event);
                        self.apply_event(redo_event);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn render_popups(&mut self, ui: &imgui::Ui) {
        self.render_error_popup(ui);
        self.render_success_popup(ui);
    }

    /// Get all the outputs defined in the node editor.
    pub fn outputs(&self) -> Vec<(cake::OutputId, String)> {
        self.dst
            .outputs_iter()
            .filter(|(_, (some_output, _))| some_output.is_some())
            .map(|(id, (_, name))| (*id, name.clone()))
            .collect()
    }

    fn render_error_popup(&mut self, ui: &imgui::Ui) {
        if !self.error_stack.is_empty() {
            ui.open_popup(format!("Error!"));
        }
        ui.popup_modal(format!("Error!")).build(ui, || {
            let stack = ui.push_text_wrap_pos_with_pos(400.0);
            let e = &self.error_stack[self.error_stack.len() - 1];
            ui.text_wrapped(&ImString::new(format!("{}", e)));
            stack.pop(ui);
            if !ui.is_window_hovered() && ui.is_mouse_clicked(imgui::MouseButton::Left) {
                self.error_stack.pop();
                ui.close_current_popup();
            }
        });
    }
    fn render_success_popup(&mut self, ui: &imgui::Ui) {
        if self.error_stack.is_empty() && !self.success_stack.is_empty() {
            ui.open_popup(format!("Success!"));
        }
        ui.popup_modal(format!("Success!")).build(ui, || {
            {
                let msg = &self.success_stack[self.success_stack.len() - 1];
                ui.text(msg);
            }
            if !ui.is_window_hovered() && ui.is_mouse_clicked(imgui::MouseButton::Left) {
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
        attaching: &mut Option<(cake::OutputId, TransformIdx, usize)>,
    ) where
        ED: ConstantEditor<T>,
    {
        const MACRO_WINDOW_DEFAULT_SIZE: [f32; 2] = [900.0, 600.0];

        let mut macros_to_edit = vec![];
        let macros = &mut self.macros;
        let import_macro = &mut self.import_macro;
        let mut import_macro_focus = false;
        for (i, node_edit) in self.nodes_edit.iter_mut().enumerate() {
            let mut opened = node_edit.opened;
            if opened {
                if node_edit.focus {
                    unsafe { imgui::sys::igSetNextWindowFocus() };
                    node_edit.focus = false;
                }
                imgui::Window::new(&imgui::ImString::new(format!(
                    "Macro editor: '{}'###{}",
                    node_edit.handle.name(),
                    i,
                )))
                .size(MACRO_WINDOW_DEFAULT_SIZE, imgui::Condition::FirstUseEver)
                .opened(&mut opened)
                .build(ui, || {
                    let events = {
                        let lock = node_edit.handle.read();
                        let dst = lock.dst();
                        node_edit.layout.render(
                            ui,
                            dst,
                            addable_nodes,
                            macros,
                            constant_editor,
                            attaching,
                        )
                    };
                    for event in events {
                        if let event::RenderEvent::AddNewMacro = event {
                            let new_macr = macros.create_macro().clone();
                            let macro_id = new_macr.id();
                            node_edit.handle.write().dst_mut().add_owned_transform(
                                cake::Transform::from_macro(new_macr),
                                Some(macro_id),
                            );
                        } else if let event::RenderEvent::EditNode(node_id) = event {
                            if let cake::NodeId::Transform(t_idx) = node_id {
                                if let Some(t) = node_edit.handle.read().dst().get_transform(t_idx)
                                {
                                    if let cake::Algorithm::Macro { handle } = t.algorithm() {
                                        macros_to_edit.push(handle.clone());
                                    }
                                }
                            }
                        } else if let event::RenderEvent::Import = event {
                            *import_macro = Some(node_edit.handle.clone());
                            import_macro_focus = true;
                        } else {
                            use event::ProvenanceEvent;
                            use event::RenderEvent;
                            let push_event = RenderEvent::new(&event);
                            node_edit.apply_event(event);
                            let pushed = node_edit.valid_history.pop().unwrap();
                            if node_edit.valid_history.is_empty() {
                                match push_event {
                                    RenderEvent::Undo => {
                                        node_edit.valid_history.push(pushed);
                                        println!("Cannot Undo");
                                        continue;
                                    }
                                    _ => {}
                                }
                            }
                            if node_edit.redo_stack.is_empty() {
                                match push_event {
                                    RenderEvent::Redo => {
                                        node_edit.valid_history.push(pushed);
                                        println!("Cannot Redo");
                                        continue;
                                    }
                                    _ => {}
                                }
                            }
                            match (push_event, &pushed) {
                                (
                                    RenderEvent::Connect(_, _),
                                    ProvenanceEvent::Connect(_, _, Ok(())),
                                )
                                | (
                                    RenderEvent::Disconnect(_, _),
                                    ProvenanceEvent::Disconnect(_, _, Ok(())),
                                )
                                | (
                                    RenderEvent::AddTransform(_),
                                    ProvenanceEvent::AddTransform(_, _),
                                )
                                | (RenderEvent::CreateOutput, ProvenanceEvent::CreateOutput(_))
                                | (
                                    RenderEvent::AddConstant(_),
                                    ProvenanceEvent::AddConstant(_, _),
                                )
                                | (
                                    RenderEvent::SetConstant(_, _),
                                    ProvenanceEvent::SetConstant(_, _, _, Ok(())),
                                )
                                | (
                                    RenderEvent::WriteDefaultInput { .. },
                                    ProvenanceEvent::WriteDefaultInput(_, _, _, _, Ok(())),
                                )
                                | (
                                    RenderEvent::RemoveNode(_),
                                    ProvenanceEvent::RemoveNode(_, _, _, _, _, _),
                                )
                                | (RenderEvent::Export, ProvenanceEvent::Export(_, _, Ok(())))
                                | (RenderEvent::AddMacro(_), ProvenanceEvent::AddMacro(_, _))
                                | (
                                    RenderEvent::ChangeOutputName(_, _),
                                    ProvenanceEvent::ChangeOutputName(_, _, _),
                                ) => {
                                    node_edit.redo_stack.clear();
                                    node_edit.valid_history.push(pushed);
                                }
                                (RenderEvent::Import, ProvenanceEvent::Import(_, _, _, Ok(())))
                                | (RenderEvent::AddNewMacro, ProvenanceEvent::AddNewMacro(_))
                                | (RenderEvent::EditNode(_), ProvenanceEvent::EditNode(_)) => {
                                    /* Defined Individually, nothing to do here. */
                                }
                                //Undo by looking at the previous operation, the opposite event is executed.
                                (RenderEvent::Undo, ProvenanceEvent::Connect(o, i, Ok(()))) => {
                                    let alternative_event = RenderEvent::<T, E>::Disconnect(*o, *i);
                                    node_edit.apply_event(alternative_event);
                                    node_edit.valid_history.pop();
                                    node_edit.redo_stack.push(pushed);
                                }
                                (RenderEvent::Undo, ProvenanceEvent::Disconnect(o, i, Ok(()))) => {
                                    let alternative_event = RenderEvent::<T, E>::Connect(*o, *i);
                                    node_edit.apply_event(alternative_event);
                                    node_edit.valid_history.pop();
                                    node_edit.redo_stack.push(pushed);
                                }
                                (RenderEvent::Undo, ProvenanceEvent::AddTransform(_, t_idx))
                                | (RenderEvent::Undo, ProvenanceEvent::AddConstant(_, t_idx)) => {
                                    let node = cake::NodeId::Transform(*t_idx);
                                    let alternative_event = RenderEvent::<T, E>::RemoveNode(node);
                                    node_edit.apply_event(alternative_event);
                                    node_edit.valid_history.pop();
                                    node_edit.redo_stack.push(pushed);
                                }
                                (RenderEvent::Undo, ProvenanceEvent::CreateOutput(output_id)) => {
                                    let node = cake::NodeId::Output(*output_id);
                                    let alternative_event = RenderEvent::<T, E>::RemoveNode(node);
                                    node_edit.apply_event(alternative_event);
                                    node_edit.valid_history.pop();
                                    node_edit.redo_stack.push(pushed);
                                }
                                (
                                    RenderEvent::Undo,
                                    ProvenanceEvent::SetConstant(t_idx, before, _, Ok(())),
                                ) => {
                                    let alternative_event = RenderEvent::<T, E>::SetConstant(
                                        *t_idx,
                                        Box::new(before.clone().unwrap()),
                                    );
                                    node_edit.apply_event(alternative_event);
                                    node_edit.valid_history.pop();
                                    node_edit.redo_stack.push(pushed);
                                }
                                (
                                    RenderEvent::Undo,
                                    ProvenanceEvent::WriteDefaultInput(
                                        t_idx,
                                        input_index,
                                        before,
                                        _,
                                        Ok(()),
                                    ),
                                ) => {
                                    let alternative_event =
                                        RenderEvent::<T, E>::WriteDefaultInput {
                                            t_idx: *t_idx,
                                            input_index: *input_index,
                                            val: Box::new(before.clone().unwrap()),
                                        };
                                    node_edit.apply_event(alternative_event);
                                    node_edit.valid_history.pop();
                                    node_edit.redo_stack.push(pushed);
                                }
                                (
                                    RenderEvent::Undo,
                                    ProvenanceEvent::RemoveNode(
                                        cake::NodeId::Transform(_),
                                        t,
                                        _,
                                        d_i,
                                        i_c,
                                        o_c,
                                    ),
                                ) => {
                                    node_edit.apply_event(RenderEvent::<T, E>::AddOwnedTransform(
                                        t.clone(),
                                        d_i.clone(),
                                        i_c.clone(),
                                        o_c.clone(),
                                    ));
                                    node_edit.valid_history.pop();
                                    node_edit.redo_stack.push(pushed);
                                }
                                (
                                    RenderEvent::Undo,
                                    ProvenanceEvent::RemoveNode(
                                        cake::NodeId::Output(output_id),
                                        _,
                                        name,
                                        _,
                                        i_c,
                                        _,
                                    ),
                                ) => {
                                    if i_c.len() == 1 {
                                        let mut lock = node_edit.handle.write();
                                        let dst = lock.dst_mut();
                                        if let Some(output) = i_c.get(0).unwrap() {
                                            dst.attach_output_with_id_name(
                                                *output,
                                                *output_id,
                                                name.clone().unwrap(),
                                            );
                                        } else {
                                            dst.create_output_with_id_name(
                                                *output_id,
                                                name.clone().unwrap(),
                                            );
                                        }
                                    }
                                    node_edit.redo_stack.push(pushed);
                                }
                                (
                                    RenderEvent::Undo,
                                    ProvenanceEvent::Import(_, Some(before_dst), _, Ok(())),
                                ) => {
                                    *node_edit.handle.write().dst_mut() = before_dst.clone();
                                    node_edit.redo_stack.push(pushed);
                                }
                                (RenderEvent::Undo, ProvenanceEvent::Export(_, _, _)) => {
                                    println!("Export undo skipped.");
                                    node_edit.redo_stack.push(pushed);
                                }
                                (RenderEvent::Undo, ProvenanceEvent::AddNewMacro(t_idx)) => {
                                    let node = cake::NodeId::Transform(*t_idx);
                                    let alternative_event = RenderEvent::<T, E>::RemoveNode(node);
                                    node_edit.apply_event(alternative_event);
                                    node_edit.valid_history.pop();
                                    node_edit.redo_stack.push(pushed);
                                }
                                (RenderEvent::Undo, ProvenanceEvent::AddMacro(_, t_idx)) => {
                                    let node = cake::NodeId::Transform(*t_idx);
                                    let alternative_event = RenderEvent::<T, E>::RemoveNode(node);
                                    node_edit.apply_event(alternative_event);
                                    node_edit.valid_history.pop();
                                    node_edit.redo_stack.push(pushed);
                                }
                                (RenderEvent::Undo, ProvenanceEvent::EditNode(_)) => {
                                    println!("EditNode skipped.");
                                    node_edit.redo_stack.push(pushed);
                                }
                                (
                                    RenderEvent::Undo,
                                    ProvenanceEvent::ChangeOutputName(node_id, before_name, _),
                                ) => {
                                    let alternative_event = RenderEvent::<T, E>::ChangeOutputName(
                                        *node_id,
                                        before_name.clone(),
                                    );
                                    node_edit.apply_event(alternative_event);
                                    node_edit.valid_history.pop();
                                    node_edit.redo_stack.push(pushed);
                                }
                                (RenderEvent::Redo, _) => {
                                    if let Some(redo_event) = node_edit.redo_stack.pop() {
                                        node_edit.valid_history.push(pushed);
                                        let redo_event = RenderEvent::<T, E>::from(redo_event);
                                        node_edit.apply_event(redo_event);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                });
                node_edit.opened = opened;
                node_edit.layout.is_macro = true;
            }

            for error in node_edit.error_stack.drain(..) {
                self.error_stack.push(Box::new(error));
            }
            self.success_stack.extend(node_edit.success_stack.drain(..));
        }

        for handle in macros_to_edit {
            open_macro_editor(&mut self.nodes_edit, handle);
        }

        let mut opened = true;
        if let Some(handle) = import_macro {
            if import_macro_focus {
                unsafe { imgui::sys::igSetNextWindowFocus() };
            }
            let mut selected_path = None;
            let mut cancelled = false;
            let mouse_pos = ui.io().mouse_pos;
            imgui::Window::new(&imgui::ImString::new(format!(
                "Import macro in '{}'",
                handle.name(),
            )))
            .opened(&mut opened)
            .save_settings(false)
            .position(mouse_pos, imgui::Condition::Appearing)
            .size([400.0, 410.0], imgui::Condition::Appearing)
            .build(ui, || {
                imgui::ChildWindow::new("import_fileexplorer")
                    .size([0.0, 350.0])
                    .horizontal_scrollbar(true)
                    .build(ui, || {
                        if let Ok((Some(path), _)) =
                            ui.file_explorer(imgui_file_explorer::CURRENT_FOLDER, &["macro"])
                        {
                            selected_path = Some(path);
                        }
                    });
                if ui.button(format!("Cancel")) {
                    cancelled = true;
                }
            });
            if cancelled {
                opened = false;
            } else if let Some(path) = selected_path.take() {
                match fs::File::open(path) {
                    Ok(file) => {
                        let editor: Result<SerialInnerEditorStandAlone<T>, _> =
                            ron::de::from_reader(file);
                        match editor {
                            Ok(editor) => match editor.into_inner_node_editor() {
                                Ok(mut editor) => {
                                    if let Some(same_id_macr) = macros
                                        .macros()
                                        .find(|h| h.id() == editor.handle.id() && h != &handle)
                                    {
                                        let msg = format!("Other macro '{}' with same ID '{}' already loaded... Replace it.",
                                            same_id_macr.name(),
                                            same_id_macr.id().to_hyphenated());
                                        eprintln!("{}", &msg);
                                        self.success_stack.push(ImString::new(msg));
                                        *same_id_macr.write() = editor.handle.read().clone();
                                        editor.handle = same_id_macr.clone();
                                        if let Some(node_edit_idx) =
                                            self.nodes_edit.iter().position(|node_edit| {
                                                node_edit.handle.id() == same_id_macr.id()
                                            })
                                        {
                                            self.nodes_edit.remove(node_edit_idx);
                                        }
                                    } else {
                                        *handle.write() = editor.handle.read().clone();
                                        editor.handle = handle.clone();
                                    }

                                    if let Some(node_edit) = self
                                        .nodes_edit
                                        .iter_mut()
                                        .find(|node_edit| &node_edit.handle == handle)
                                    {
                                        editor.focus = true;
                                        editor.opened = true;
                                        *node_edit = editor;
                                    } else {
                                        eprintln!("Could not update macro editor. Not found...");
                                    }
                                }
                                Err(e) => self.error_stack.push(Box::new(e)),
                            },
                            Err(e) => self
                                .error_stack
                                .push(Box::new(export::ImportError::DeserializationError(e))),
                        }
                    }
                    Err(e) => {
                        use std::sync::Arc;
                        self.error_stack
                            .push(Box::new(export::ImportError::IOError(Arc::new(e))));
                    }
                }
                opened = false;
            }
        }
        if !opened {
            *import_macro = None;
        }
    }
}

fn open_macro_editor<T, E>(
    nodes_edit: &mut Vec<InnerNodeEditor<T, E>>,
    handle: cake::macros::MacroHandle<'static, T, E>,
) {
    let found = if let Some(editor) = nodes_edit
        .iter_mut()
        .find(|node_edit| node_edit.handle == handle)
    {
        editor.opened = true;
        editor.focus = true;
        true
    } else {
        false
    };
    if !found {
        nodes_edit.push(InnerNodeEditor::new(handle));
    }
}

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
    fn connect(
        &mut self,
        output: cake::Output,
        input_slot: cake::InputSlot,
        leave_provenance: bool,
    ) {
        match input_slot {
            cake::InputSlot::Transform(input) => {
                if let Err(e) = self.dst.connect(output, input) {
                    eprintln!("{:?}", e);
                    if leave_provenance {
                        self.valid_history.push(event::ProvenanceEvent::Connect(
                            output,
                            input_slot,
                            Err(e.clone()),
                        ));
                    }
                    self.error_stack.push(Box::new(e));
                } else {
                    if leave_provenance {
                        self.valid_history.push(event::ProvenanceEvent::Connect(
                            output,
                            input_slot,
                            Ok(()),
                        ));
                    }
                }
            }
            cake::InputSlot::Output(output_id) => {
                if let Some(cake::Node::Output((_, name))) =
                    self.dst.get_node(&cake::NodeId::Output(output_id))
                {
                    self.dst.update_output(output_id, output, name);
                    if leave_provenance {
                        self.valid_history.push(event::ProvenanceEvent::Connect(
                            output,
                            input_slot,
                            Ok(()),
                        ));
                    }
                } else {
                    let e = cake::DSTError::InvalidInput(format!(
                        "{:?} does not exist in this graph!",
                        output_id
                    ));
                    if leave_provenance {
                        self.valid_history.push(event::ProvenanceEvent::Connect(
                            output,
                            input_slot,
                            Err(e),
                        ));
                    }
                }
            }
        }
    }
    fn disconnect(&mut self, output: cake::Output, input_slot: cake::InputSlot) {
        match input_slot {
            cake::InputSlot::Transform(input) => self.dst.disconnect(&output, &input),
            cake::InputSlot::Output(output_id) => self.dst.detach_output(&output, &output_id),
        }
        self.valid_history.push(event::ProvenanceEvent::Disconnect(
            output,
            input_slot,
            Ok(()),
        ))
    }
    fn add_transform(&mut self, t: &'static cake::Transform<'static, T, E>) {
        let t_idx = self.dst.add_transform(t, None);
        self.valid_history
            .push(event::ProvenanceEvent::AddTransform(t, t_idx));
    }
    fn add_owned_transform(
        &mut self,
        t: Option<cake::Transform<'static, T, E>>,
        d_i: Vec<Option<T>>,
        i_c: Vec<Option<cake::Output>>,
        o_c: Vec<(cake::Output, cake::InputSlot)>,
    ) {
        if let Some(t) = t {
            let t_idx = self.dst.add_owned_transform(t.clone(), None);
            for (input_idx, default_input) in d_i.iter().enumerate() {
                if let Some(val) = default_input {
                    self.write_default_input(t_idx, input_idx, Box::new(val.clone()), false);
                }
            }
            for (input_idx, input_connect) in i_c.iter().enumerate() {
                if let Some(i_c) = input_connect {
                    let input_slot = cake::InputSlot::Transform(cake::Input::new(t_idx, input_idx));
                    self.connect(*i_c, input_slot, false);
                }
            }
            for o_cs in &o_c {
                self.connect(o_cs.0, o_cs.1, false);
            }

            self.valid_history
                .push(event::ProvenanceEvent::AddOwnedTransform(
                    Some(t),
                    t_idx,
                    d_i,
                    i_c,
                    o_c,
                ));
        }
    }
    fn create_output(&mut self) {
        let output_id = self.dst.create_output();
        self.valid_history
            .push(event::ProvenanceEvent::CreateOutput(output_id));
    }
    fn add_constant(&mut self, constant_type: &'static str) {
        let constant = cake::Transform::new_constant(T::default_for(constant_type));
        let t_idx = self.dst.add_owned_transform(constant, None);
        self.valid_history
            .push(event::ProvenanceEvent::AddConstant(constant_type, t_idx));
    }
    fn set_constant(&mut self, t_idx: cake::TransformIdx, c: Box<T>) {
        if let Some(t) = self.dst.get_transform_mut(t_idx) {
            if let cake::Algorithm::Constant(ref constant) = t.clone().algorithm() {
                t.set_constant(*c.clone());
                self.valid_history.push(event::ProvenanceEvent::SetConstant(
                    t_idx,
                    Some(constant.clone()),
                    c,
                    Ok(()),
                ));
            }
        } else {
            eprintln!("Transform {:?} was not found.", t_idx);
            self.valid_history
                .push(event::ProvenanceEvent::SetConstant(t_idx, None, c, Err(())));
        }
    }
    fn write_default_input(
        &mut self,
        t_idx: cake::TransformIdx,
        input_index: usize,
        val: Box<T>,
        leave_provenance: bool,
    ) {
        if let Some(mut inputs) = self.dst.get_default_inputs_mut(t_idx) {
            let before = inputs.read(input_index);
            inputs.write(input_index, *val.clone());
            if leave_provenance {
                self.valid_history
                    .push(event::ProvenanceEvent::WriteDefaultInput(
                        t_idx,
                        input_index,
                        before,
                        val,
                        Ok(()),
                    ));
            }
        } else {
            eprintln!("Transform {:?} was not found.", t_idx);
            if leave_provenance {
                self.valid_history
                    .push(event::ProvenanceEvent::WriteDefaultInput(
                        t_idx,
                        input_index,
                        None,
                        val,
                        Err(()),
                    ));
            }
        }
    }
    fn remove_node(&mut self, node_id: cake::NodeId) {
        if let cake::NodeId::Transform(t_idx) = node_id {
            let t = self.dst.get_transform(t_idx).unwrap();
            let default_inputs = self.dst.get_default_inputs(t_idx).unwrap().to_vec();
            let inputside_connects = self.dst.outputs_attached_to_transform(t_idx).unwrap();
            let mut outputside_connects = Vec::new();
            if let Some(outputs) = self.dst.outputs_of_transformation(t_idx) {
                for output in outputs {
                    if let Some(inputs) = self
                        .dst
                        .inputs_attached_to(&output)
                        .map(|inputs| inputs.cloned().collect::<Vec<_>>())
                    {
                        for input in inputs {
                            outputside_connects.push((output, cake::InputSlot::Transform(input)));
                        }
                    }
                }
            }
            self.valid_history.push(event::ProvenanceEvent::RemoveNode(
                node_id,
                Some(t.clone()),
                None,
                default_inputs,
                inputside_connects,
                outputside_connects,
            ));
        } else if let cake::NodeId::Output(_) = node_id {
            let o = self.dst.get_node(&node_id).unwrap();
            if let cake::Node::Output((output, name)) = o {
                let output = if let Some(o) = output { Some(*o) } else { None };
                self.valid_history.push(event::ProvenanceEvent::RemoveNode(
                    node_id,
                    None,
                    Some(name),
                    vec![],
                    vec![output],
                    vec![],
                ));
            }
        }

        self.dst.remove_node(&node_id);
        self.layout.node_states.remove_node(&node_id);
        if self.layout.active_node == Some(node_id) {
            self.layout.active_node.take();
        }
    }
    fn import(&mut self) {
        let before_dst = self.dst.clone();
        if let Some(path) = self.layout.import_path.take() {
            if let Err(e) = self.import_from_file(path.clone()) {
                eprintln!("Error on import! {}", e);
                self.valid_history.push(event::ProvenanceEvent::Import(
                    Some(path),
                    Some(before_dst),
                    self.dst.clone(),
                    Err(e.clone()),
                ));
                self.error_stack.push(Box::new(e));
            } else {
                self.valid_history.push(event::ProvenanceEvent::Import(
                    Some(path.clone()),
                    Some(before_dst),
                    self.dst.clone(),
                    Ok(()),
                ));
                for node_edit in self.nodes_edit.iter_mut() {
                    node_edit.valid_history.push(event::ProvenanceEvent::Import(
                        Some(path.clone()),
                        None,
                        node_edit.handle.read().dst().clone(),
                        Ok(()),
                    ))
                }
            }
            self.layout.import_path = None;
        }
    }
    fn export(&mut self) {
        if let Some(path) = self.layout.export_path.take() {
            if let Err(e) = self.export_to_file(path.clone()) {
                eprintln!("Error on export! {}", e);
                self.valid_history.push(event::ProvenanceEvent::Export(
                    path,
                    self.dst.clone(),
                    Err(e.clone()),
                ));
                self.error_stack.push(Box::new(e));
            } else {
                self.valid_history.push(event::ProvenanceEvent::Export(
                    path.clone(),
                    self.dst.clone(),
                    Ok(()),
                ));
                self.success_stack.push(ImString::new(format!(
                    "Editor content was exported with success to '{:?}'!",
                    path
                )));
            }
        }
    }
    fn add_new_macro(&mut self) {
        let t_idx = self.dst.add_owned_transform(
            cake::Transform::from_macro(self.macros.create_macro().clone()),
            None,
        );
        self.valid_history
            .push(event::ProvenanceEvent::AddNewMacro(t_idx));
    }
    fn add_macro(&mut self, handle: cake::macros::MacroHandle<'static, T, E>) {
        let t_idx = self
            .dst
            .add_owned_transform(cake::Transform::from_macro(handle.clone()), None);
        self.valid_history
            .push(event::ProvenanceEvent::AddMacro(handle, t_idx));
    }
    fn edit_node(&mut self, node_id: cake::NodeId) {
        if let cake::NodeId::Transform(t_idx) = node_id {
            if let Some(t) = self.dst.get_transform(t_idx) {
                if let cake::Algorithm::Macro { handle } = t.algorithm() {
                    open_macro_editor(&mut self.nodes_edit, handle.clone());
                    self.valid_history
                        .push(event::ProvenanceEvent::EditNode(node_id));
                }
            }
        }
    }
    fn change_output_name(&mut self, node_id: cake::NodeId, name: String) {
        if let cake::NodeId::Output(output_id) = node_id {
            let node = self.dst.get_node(&node_id);
            if let Some(cake::Node::Output((_, before_name))) = node {
                self.valid_history
                    .push(event::ProvenanceEvent::ChangeOutputName(
                        node_id,
                        before_name,
                        name.clone(),
                    ));
                self.dst.change_output_name(output_id, name);
            }
        }
    }
}

impl<T, E> ApplyRenderEvent<T, E> for InnerNodeEditor<T, E>
where
    T: Clone + cake::ConvertibleVariants + cake::DefaultFor + serde::Serialize,
{
    fn connect(
        &mut self,
        output: cake::Output,
        input_slot: cake::InputSlot,
        leave_provenance: bool,
    ) {
        let mut lock = self.handle.write();
        let dst = lock.dst_mut();
        match input_slot {
            cake::InputSlot::Transform(input) => {
                if let Err(e) = dst.connect(output, input) {
                    eprintln!("Cannot connect in macro: {:?}", e);
                    if leave_provenance {
                        self.valid_history.push(event::ProvenanceEvent::Connect(
                            output,
                            input_slot,
                            Err(e.clone()),
                        ));
                    }
                    self.error_stack
                        .push(InnerEditorError::IncorrectNodeConnection(e));
                } else {
                    if leave_provenance {
                        self.valid_history.push(event::ProvenanceEvent::Connect(
                            output,
                            input_slot,
                            Ok(()),
                        ));
                    }
                }
            }
            cake::InputSlot::Output(output_id) => {
                if let Some(cake::Node::Output((_, name))) =
                    dst.get_node(&cake::NodeId::Output(output_id))
                {
                    if leave_provenance {
                        self.valid_history.push(event::ProvenanceEvent::Connect(
                            output,
                            input_slot,
                            Ok(()),
                        ));
                    }
                    dst.update_output(output_id, output, name)
                }
            }
        }
    }
    fn disconnect(&mut self, output: cake::Output, input_slot: cake::InputSlot) {
        let mut lock = self.handle.write();
        let dst = lock.dst_mut();
        match input_slot {
            cake::InputSlot::Transform(input) => dst.disconnect(&output, &input),
            cake::InputSlot::Output(output_id) => dst.detach_output(&output, &output_id),
        }
        self.valid_history.push(event::ProvenanceEvent::Disconnect(
            output,
            input_slot,
            Ok(()),
        ))
    }
    fn add_transform(&mut self, t: &'static cake::Transform<'static, T, E>) {
        let handle_id = self.handle.id();
        let t_idx = self
            .handle
            .write()
            .dst_mut()
            .add_transform(t, Some(handle_id));
        self.valid_history
            .push(event::ProvenanceEvent::AddTransform(t, t_idx));
    }
    fn add_owned_transform(
        &mut self,
        t: Option<cake::Transform<'static, T, E>>,
        d_i: Vec<Option<T>>,
        i_c: Vec<Option<cake::Output>>,
        o_c: Vec<(cake::Output, cake::InputSlot)>,
    ) {
        if let Some(t) = t {
            let handle_id = self.handle.id();
            let mut lock = self.handle.write();
            let dst = lock.dst_mut();
            let t_idx = dst.add_owned_transform(t.clone(), Some(handle_id));
            drop(lock);
            for (input_idx, default_input) in d_i.iter().enumerate() {
                if let Some(val) = default_input {
                    self.write_default_input(t_idx, input_idx, Box::new(val.clone()), false);
                }
            }
            for (input_idx, input_connect) in i_c.iter().enumerate() {
                if let Some(i_c) = input_connect {
                    let input_slot = cake::InputSlot::Transform(cake::Input::new(t_idx, input_idx));
                    self.connect(*i_c, input_slot, false);
                }
            }
            for o_cs in &o_c {
                self.connect(o_cs.0, o_cs.1, false);
            }

            self.valid_history
                .push(event::ProvenanceEvent::AddOwnedTransform(
                    Some(t),
                    t_idx,
                    d_i,
                    i_c,
                    o_c,
                ));
        }
    }
    fn create_output(&mut self) {
        let output_id = self.handle.write().dst_mut().create_output();
        self.valid_history
            .push(event::ProvenanceEvent::CreateOutput(output_id));
    }
    fn add_constant(&mut self, constant_type: &'static str) {
        let constant = cake::Transform::new_constant(T::default_for(constant_type));
        let handle_id = self.handle.id();
        let t_idx = self
            .handle
            .write()
            .dst_mut()
            .add_owned_transform(constant, Some(handle_id));
        self.valid_history
            .push(event::ProvenanceEvent::AddConstant(constant_type, t_idx));
    }
    fn set_constant(&mut self, t_idx: cake::TransformIdx, c: Box<T>) {
        let mut lock = self.handle.write();
        let dst = lock.dst_mut();
        if let Some(t) = dst.get_transform_mut(t_idx) {
            if let cake::Algorithm::Constant(ref constant) = t.clone().algorithm() {
                t.set_constant(*c.clone());
                self.valid_history.push(event::ProvenanceEvent::SetConstant(
                    t_idx,
                    Some(constant.clone()),
                    c,
                    Ok(()),
                ));
            }
        } else {
            eprintln!("Transform {:?} was not found in macro.", t_idx,);
        }
    }
    fn write_default_input(
        &mut self,
        t_idx: cake::TransformIdx,
        input_index: usize,
        val: Box<T>,
        leave_provenance: bool,
    ) {
        let mut lock = self.handle.write();
        let dst = lock.dst_mut();
        if let Some(mut inputs) = dst.get_default_inputs_mut(t_idx) {
            let before = inputs.read(input_index);
            inputs.write(input_index, *val.clone());
            if leave_provenance {
                self.valid_history
                    .push(event::ProvenanceEvent::WriteDefaultInput(
                        t_idx,
                        input_index,
                        before,
                        val,
                        Ok(()),
                    ));
            }
        } else {
            eprintln!("Transform {:?} was not found.", t_idx);
            if leave_provenance {
                self.valid_history
                    .push(event::ProvenanceEvent::WriteDefaultInput(
                        t_idx,
                        input_index,
                        None,
                        val,
                        Err(()),
                    ));
            }
        }
    }
    fn remove_node(&mut self, node_id: cake::NodeId) {
        let lock = self.handle.read();
        let dst = lock.dst();
        if let cake::NodeId::Transform(t_idx) = node_id {
            let t = dst.get_transform(t_idx).unwrap();
            let default_inputs = dst.get_default_inputs(t_idx).unwrap().to_vec();
            let inputside_connects = dst.outputs_attached_to_transform(t_idx).unwrap();
            let mut outputside_connects = Vec::new();
            if let Some(outputs) = dst.outputs_of_transformation(t_idx) {
                for output in outputs {
                    if let Some(inputs) = dst
                        .inputs_attached_to(&output)
                        .map(|inputs| inputs.cloned().collect::<Vec<_>>())
                    {
                        for input in inputs {
                            outputside_connects.push((output, cake::InputSlot::Transform(input)));
                        }
                    }
                }
            }
            self.valid_history.push(event::ProvenanceEvent::RemoveNode(
                node_id,
                Some(t.clone()),
                None,
                default_inputs,
                inputside_connects,
                outputside_connects,
            ));
        } else if let cake::NodeId::Output(_) = node_id {
            let o = dst.get_node(&node_id).unwrap();
            if let cake::Node::Output((output, name)) = o {
                let output = if let Some(o) = output { Some(*o) } else { None };
                self.valid_history.push(event::ProvenanceEvent::RemoveNode(
                    node_id,
                    None,
                    Some(name),
                    vec![],
                    vec![output],
                    vec![],
                ));
            }
        }
        drop(lock);
        self.handle.write().dst_mut().remove_node(&node_id);
        self.layout.node_states.remove_node(&node_id);
        if self.layout.active_node == Some(node_id) {
            self.layout.active_node.take();
        }
    }
    fn import(&mut self) {
        unreachable!("Import can only be handled in NodeEditor's context!");
    }
    fn export(&mut self) {
        let file_name = format!("{}.macro", self.handle.name());
        if let Err(e) = self.export_to_file(&file_name) {
            eprintln!("Error on export! {}", e);
            self.error_stack.push(InnerEditorError::ExportError(e));
        } else {
            self.success_stack.push(ImString::new(format!(
                "Macro content was exported with success to '{}'!",
                file_name
            )));
        }
    }
    fn add_new_macro(&mut self) {
        unreachable!("Macro can only be created in NodeEditor's context!");
    }
    fn add_macro(&mut self, handle: cake::macros::MacroHandle<'static, T, E>) {
        // FIXME: Prevent non-trivial recursive macros
        if self.handle == handle {
            self.error_stack.push(InnerEditorError::SelfDefiningMacro {
                name: self.handle.name(),
            });
        } else {
            self.handle
                .write()
                .dst_mut()
                .add_owned_transform(cake::Transform::from_macro(handle), Some(self.handle.id()));
        }
    }
    fn edit_node(&mut self, _: cake::NodeId) {
        unreachable!("Macro can only be edited in NodeEditor's context!");
    }
    fn change_output_name(&mut self, node_id: cake::NodeId, name: String) {
        let mut lock = self.handle.write();
        let dst = lock.dst_mut();
        if let cake::NodeId::Output(output_id) = node_id {
            let node = dst.get_node(&node_id);
            if let Some(cake::Node::Output((_, before_name))) = node {
                self.valid_history
                    .push(event::ProvenanceEvent::ChangeOutputName(
                        node_id,
                        before_name,
                        name.clone(),
                    ));
                dst.change_output_name(output_id, name);
            }
        }
    }
}

#[derive(Debug)]
enum InnerEditorError {
    IncorrectNodeConnection(cake::DSTError),
    SelfDefiningMacro { name: String },
    ExportError(export::ExportError),
}

impl fmt::Display for InnerEditorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use crate::InnerEditorError::*;
        match self {
            IncorrectNodeConnection(e) => write!(f, "{}", e),
            SelfDefiningMacro { name } => write!(f, "Cannot re-use macro '{}' in itself!", name),
            ExportError(e) => write!(f, "Error on export macro! {}", e),
        }
    }
}

impl error::Error for InnerEditorError {}

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
    dst: cake::macros::SerdeDSTStandAlone<T>,
    node_states: Vec<(&'e cake::NodeId, &'e node_state::NodeState)>,
    scrolling: vec2::Vec2,

    nodes_edit: Vec<SerialInnerEditor>,
}

impl<'e, T> SerialEditor<'e, T>
where
    T: Clone + cake::VariantName,
{
    fn new<E>(editor: &'e NodeEditor<T, E>) -> Self {
        Self {
            dst: cake::macros::SerdeDSTStandAlone::from(&editor.dst),
            node_states: editor.layout.node_states().iter().collect(),
            scrolling: editor.layout.scrolling().get_current(),
            nodes_edit: editor
                .nodes_edit
                .iter()
                .map(SerialInnerEditor::new)
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(bound(deserialize = "T: serde::Deserialize<'de>"))]
struct DeserEditor<T> {
    dst: cake::macros::SerdeDSTStandAlone<T>,
    node_states: Vec<(cake::NodeId, node_state::NodeState)>,
    scrolling: vec2::Vec2,

    nodes_edit: Vec<SerialInnerEditor>,
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
        let (dst, macros) = deserialized.dst.into_dst()?;
        self.dst = dst;
        self.macros = macros;

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

        // Load macro editing windows
        let mut nodes_edit = Vec::with_capacity(deserialized.nodes_edit.len());
        for node_edit in deserialized.nodes_edit {
            nodes_edit.push(node_edit.into_inner_node_editor(&self.macros)?);
        }
        self.nodes_edit = nodes_edit;

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
            import_macro: None,
            valid_history: vec![],
            redo_stack: vec![],
        }
    }
}

impl<T, E> InnerNodeEditor<T, E>
where
    T: Clone + cake::VariantName + serde::Serialize,
{
    fn export_to_buf<W: io::Write>(&self, w: &mut W) -> Result<(), export::ExportError> {
        let serializable = SerialInnerEditorStandAlone::new(self);
        let serialized = ron::ser::to_string_pretty(&serializable, Default::default())?;
        w.write_all(serialized.as_bytes())?;
        w.flush()?;
        Ok(())
    }

    fn export_to_file<P: AsRef<path::Path>>(
        &self,
        file_path: P,
    ) -> Result<(), export::ExportError> {
        let mut f = fs::File::create(file_path)?;
        self.export_to_buf(&mut f)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SerialInnerEditor {
    macro_id: cake::uuid::Uuid,
    node_states: Vec<(cake::NodeId, node_state::NodeState)>,
    scrolling: vec2::Vec2,
}

impl SerialInnerEditor {
    fn new<T, E>(editor: &InnerNodeEditor<T, E>) -> Self {
        Self {
            macro_id: editor.handle.id(),
            node_states: editor
                .layout
                .node_states()
                .iter()
                .map(|(id, state)| (*id, state.clone()))
                .collect(),
            scrolling: editor.layout.scrolling().get_current(),
        }
    }

    fn into_inner_node_editor<T, E>(
        self,
        manager: &cake::macros::MacroManager<'static, T, E>,
    ) -> Result<InnerNodeEditor<T, E>, export::ImportError> {
        if let Some(handle) = manager.get_macro(self.macro_id) {
            let mut layout = NodeEditorLayout::default();
            let node_states = {
                let mut node_states = node_state::NodeStates::new();
                for (node_id, state) in self.node_states {
                    node_states.insert(node_id, state);
                }
                node_states
            };
            let scrolling = scrolling::Scrolling::new(self.scrolling);
            layout.import(node_states, scrolling);
            Ok(InnerNodeEditor {
                handle: handle.clone(),
                layout,
                opened: false,
                focus: false,
                error_stack: vec![],
                success_stack: vec![],
                valid_history: vec![],
                redo_stack: vec![],
            })
        } else {
            Err(export::ImportError::DSTError(
                cake::ImportError::MacroNotFound(self.macro_id),
            ))
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SerialInnerEditorStandAlone<T> {
    // FIXME: Serialize and deserialize editor of sub-macros
    editor: SerialInnerEditor,
    macr: cake::macros::SerdeMacroStandAlone<T>,
}

impl<T> SerialInnerEditorStandAlone<T> {
    fn new<E>(editor: &InnerNodeEditor<T, E>) -> Self
    where
        T: Clone + cake::VariantName,
    {
        Self {
            editor: SerialInnerEditor::new(editor),
            macr: cake::macros::SerdeMacroStandAlone::from(&editor.handle),
        }
    }

    fn into_inner_node_editor<E>(self) -> Result<InnerNodeEditor<T, E>, export::ImportError>
    where
        T: Clone + cake::VariantName + cake::ConvertibleVariants + cake::NamedAlgorithms<E>,
    {
        let mut manager = cake::macros::MacroManager::new();
        manager.add_macro(self.macr.into_macro()?)?;
        self.editor.into_inner_node_editor(&manager)
    }
}
