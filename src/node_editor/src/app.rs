use std::io;
use std::collections::BTreeMap;
use std::error;

use serde::{Serialize, Deserialize};
use imgui::{Ui, ImString};

use constant_editor::ConstantEditor;
use cake::{self,Transformation, VariantName, NamedAlgorithms};


use editor::NodeEditor;
use export::ImportError;
use node_editable::{MacroEditor, DstEditor};

pub struct NodeEditorApp<'t, T: 't + Clone, E: 't, ED> {
    main: NodeEditor<'t, DstEditor<'t, T, E>, T, E, ED>,
    macros: BTreeMap<String, NodeEditor<'t, MacroEditor<'t, T, E>, T, E, ED>>,
}

impl<'t, T, E, ED> NodeEditorApp<'t, T, E, ED>
where
    T: 'static + Clone + VariantName + NamedAlgorithms<E> + for<'de> Deserialize<'de>,
    E: 'static,
    ED: Default,
{
    pub fn from_export_buf<R>(
        r: R,
        addable_nodes: &'t [&'t Transformation<T, E>],
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

impl<'t, T, E, ED> NodeEditorApp<'t, T, E, ED>
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
    E: 'static + error::Error
{
    pub fn render(&mut self, ui: &Ui) {
        self.main.render(ui);

        for (macro_name, macr) in self.macros.iter_mut() {
            // TODO: Add boolean flag (if editing show)
            let popup_name = ImString::new(macro_name.clone());
            ui.open_popup(&popup_name);
            ui.popup_modal(&popup_name).build(|| {
                macr.render(ui);
            });
        }
    }
}
