use std::collections::BTreeMap;
use std::error;
use std::io;

use imgui::{ImString, Ui};
use serde::{Deserialize, Serialize};

use cake::{self, MacroEvaluationError, NamedAlgorithms, Transformation, VariantName};
use constant_editor::ConstantEditor;

use compute::ComputeResult;
use editor::NodeEditor;
use export::ImportError;
use node_editable::{DstEditor, MacroEditor};

pub struct NodeEditorApp<T: 'static + Clone, E: 'static, ED> {
    main: NodeEditor<DstEditor<T, E>, T, E, ED>,
    macros: BTreeMap<String, NodeEditor<MacroEditor<T, E>, T, E, ED>>,
}

impl<T, E, ED> NodeEditorApp<T, E, ED>
where
    T: Clone,
    ED: Default,
{
    pub fn new(addable_nodes: &'static [&'static Transformation<T, E>], ed: ED) -> Self {
        Self {
            main: NodeEditor::new(addable_nodes, ed),
            macros: BTreeMap::new(),
        }
    }
}

impl<T, E, ED> NodeEditorApp<T, E, ED>
where
    T: 'static + Clone + VariantName + NamedAlgorithms<E> + for<'de> Deserialize<'de>,
    E: 'static,
    ED: Default,
{
    pub fn from_export_buf<R>(
        r: R,
        addable_nodes: &'static [&'static Transformation<T, E>],
        ed: ED,
    ) -> Result<Self, ImportError<E>>
    where
        R: io::Read,
    {
        let editor = NodeEditor::from_export_buf(r, addable_nodes, ed)?;
        Ok(Self {
            main: editor,
            macros: BTreeMap::new(),
        })
    }
}

impl<T, E, ED> NodeEditorApp<T, E, ED>
where
    T: 'static
        + Clone
        + cake::EditableVariants
        + cake::NamedAlgorithms<E>
        + cake::VariantName
        + cake::DefaultFor
        + Serialize
        + for<'de> Deserialize<'de>,
    ED: ConstantEditor<T>,
    E: 'static + error::Error,
{
    pub fn render(&mut self, ui: &Ui) {
        let event = self.main.render(ui);

        if let Some(new_macro) = event.new_macro {
            let editor = MacroEditor::new(new_macro);
            let macro_editor = self.main.spawn_editor(editor);
            // TODO: Set name and see name in !
            let name = String::from("Default name");
            self.macros.insert(name, macro_editor);
        }

        for (macro_name, macr) in self.macros.iter_mut() {
            // TODO: Add boolean flag (if editing show)
            let popup_name = ImString::new(macro_name.clone());
            if macr.inner.editing {
                ui.open_popup(&popup_name);
                // Another window for macro editing is way better...
                ui.popup_modal(&popup_name).build(|| {
                    macr.render(ui);
                    use imgui::ImGuiKey;
                    let escape_index = ui.imgui().get_key_index(ImGuiKey::Escape);
                    if ui.imgui().is_key_down(escape_index) {
                        macr.inner.editing = false;
                        ui.close_current_popup();
                    }
                });
            }
        }
    }

    pub fn outputs(&self) -> Vec<cake::OutputId> {
        self.main.outputs()
    }
}

impl<T: 'static, E: 'static, ED> NodeEditorApp<T, E, ED>
where
    T: Clone + cake::VariantName + Send + Sync,
    E: Send + From<MacroEvaluationError<E>>,
{
    pub unsafe fn compute_output(&self, id: cake::OutputId) -> ComputeResult<T, E> {
        self.main.compute_output(id)
    }
}

impl<T, E, ED> NodeEditorApp<T, E, ED>
where
    T: Clone + PartialEq,
{
    pub fn update_constant_node(&mut self, id: cake::TransformIdx, val: Vec<T>) {
        self.main.update_constant_node(id, val)
    }
}

impl<T, E, ED> NodeEditorApp<T, E, ED>
where
    T: Clone + cake::VariantName,
{
    pub fn create_constant_node(&mut self, t: T) -> cake::TransformIdx {
        self.main.create_constant_node(t)
    }
}

impl<T, E, ED> NodeEditorApp<T, E, ED>
where
    T: Clone,
{
    pub fn constant_node_value(&self, id: cake::TransformIdx) -> Option<&[T]> {
        self.main.constant_node_value(id)
    }
}
