use std::io;
use std::collections::BTreeMap;
use std::error;

use serde::Deserialize;

use cake::{Transformation, VariantName, NamedAlgorithms};

use editor::NodeEditor;
use export::ImportError;
use node_editable::{MacroEditor, DstEditor};

pub struct NodeEditorApp<'t, T: 't + Clone, E: 't, ED> {
    main: NodeEditor<'t, DstEditor<'t, T, E>, T, E, ED>,
    macros: BTreeMap<String, NodeEditor<'t, MacroEditor<'t, T, E>, T, E, ED>>,
    error_stack: Vec<Box<error::Error>>,
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
            error_stack: Vec::new(),
        })
    }

}
