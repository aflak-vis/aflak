//! A node editor library built on top of `aflak_cake` and `imgui`.
//!
//! For development you will want to check the
//! [NodeEditor](struct.NodeEditor.html) struct.
extern crate aflak_cake as cake;
#[macro_use]
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

use cake::{Future, TransformIdx};
use imgui::ImString;
use imgui_file_explorer::UiFileExplorer;

pub use constant_editor::ConstantEditor;
use event::ApplyRenderEvent;
use layout::NodeEditorLayout;

/// The node editor instance.
pub struct NodeEditor<T: 'static, E: 'static> {
    pub dst: cake::DST<'static, T, E>,
    output_results: collections::BTreeMap<cake::OutputId, ComputationState<T, E>>,
    cache: cake::Cache<T, cake::compute::ComputeError<E>>,
    macros: cake::macros::MacroManager<'static, T, E>,
    layout: NodeEditorLayout<T, E>,
    error_stack: Vec<Box<dyn error::Error>>,
    success_stack: Vec<ImString>,

    nodes_edit: Vec<InnerNodeEditor<T, E>>,
    import_macro: Option<cake::macros::MacroHandle<'static, T, E>>,
}

struct InnerNodeEditor<T: 'static, E: 'static> {
    handle: cake::macros::MacroHandle<'static, T, E>,
    layout: NodeEditorLayout<T, E>,
    opened: bool,
    focus: bool,

    error_stack: Vec<InnerEditorError>,
    success_stack: Vec<ImString>,
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
            self.apply_event(event);
        }
    }

    pub fn render_popups(&mut self, ui: &imgui::Ui) {
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
            let stack = ui.push_text_wrap_pos(400.0);
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
            ui.open_popup(im_str!("Success!"));
        }
        ui.popup_modal(im_str!("Success!")).build(|| {
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
                            node_edit
                                .handle
                                .write()
                                .dst_mut()
                                .add_owned_transform(cake::Transform::from_macro(new_macr));
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
                            node_edit.apply_event(event);
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
                imgui::ChildWindow::new(im_str!("edit"))
                    .size([0.0, 350.0])
                    .horizontal_scrollbar(true)
                    .build(ui, || {
                        if let Ok((Some(path), _)) =
                            ui.file_explorer(imgui_file_explorer::TOP_FOLDER, &["macro"])
                        {
                            selected_path = Some(path);
                        }
                    });
                if ui.button(im_str!("Cancel"), [0.0, 0.0]) {
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
                        self.error_stack
                            .push(Box::new(export::ImportError::IOError(e)));
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
    fn disconnect(&mut self, output: cake::Output, input_slot: cake::InputSlot) {
        match input_slot {
            cake::InputSlot::Transform(input) => self.dst.disconnect(&output, &input),
            cake::InputSlot::Output(output_id) => self.dst.detach_output(&output, &output_id),
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
        if let Some(path) = self.layout.import_path.take() {
            if let Err(e) = self.import_from_file(path) {
                eprintln!("Error on import! {}", e);
                self.error_stack.push(Box::new(e));
            }
            self.layout.import_path = None;
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
    fn add_macro(&mut self, handle: cake::macros::MacroHandle<'static, T, E>) {
        self.dst
            .add_owned_transform(cake::Transform::from_macro(handle));
    }
    fn edit_node(&mut self, node_id: cake::NodeId) {
        if let cake::NodeId::Transform(t_idx) = node_id {
            if let Some(t) = self.dst.get_transform(t_idx) {
                if let cake::Algorithm::Macro { handle } = t.algorithm() {
                    open_macro_editor(&mut self.nodes_edit, handle.clone());
                }
            }
        }
    }
}

impl<T, E> ApplyRenderEvent<T, E> for InnerNodeEditor<T, E>
where
    T: Clone + cake::ConvertibleVariants + cake::DefaultFor + serde::Serialize,
{
    fn connect(&mut self, output: cake::Output, input_slot: cake::InputSlot) {
        let mut lock = self.handle.write();
        let dst = lock.dst_mut();
        match input_slot {
            cake::InputSlot::Transform(input) => {
                if let Err(e) = dst.connect(output, input) {
                    eprintln!("Cannot connect in macro: {:?}", e);
                    self.error_stack
                        .push(InnerEditorError::IncorrectNodeConnection(e));
                }
            }
            cake::InputSlot::Output(output_id) => dst.update_output(output_id, output),
        }
    }
    fn disconnect(&mut self, output: cake::Output, input_slot: cake::InputSlot) {
        let mut lock = self.handle.write();
        let dst = lock.dst_mut();
        match input_slot {
            cake::InputSlot::Transform(input) => dst.disconnect(&output, &input),
            cake::InputSlot::Output(output_id) => dst.detach_output(&output, &output_id),
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
                .add_owned_transform(cake::Transform::from_macro(handle));
        }
    }
    fn edit_node(&mut self, _: cake::NodeId) {
        unreachable!("Macro can only be edited in NodeEditor's context!");
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
        use InnerEditorError::*;
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
