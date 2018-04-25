use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use cake::{DeserDST, ImportError, NamedAlgorithms, NodeId, SerialDST, VariantName};
use ron::ser;
use serde::Serialize;

use compute;
use editor::NodeEditor;
use node_state::{NodeState, NodeStates};

#[derive(Serialize)]
pub struct SerialEditor<'e, T: 'e> {
    dst: SerialDST<'e, T>,
    node_states: Vec<(&'e NodeId, &'e NodeState)>,
}

impl<'e, T> SerialEditor<'e, T>
where
    T: Clone,
{
    fn new<E, ED>(editor: &'e NodeEditor<T, E, ED>) -> Self {
        Self {
            dst: SerialDST::new(&editor.dst),
            node_states: editor.node_states.iter().collect(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct DeserEditor<T, E> {
    dst: DeserDST<T, E>,
    node_states: Vec<(NodeId, NodeState)>,
}

impl<'t, T, E, ED> NodeEditor<'t, T, E, ED>
where
    T: Clone,
{
    pub fn export(&self) -> SerialEditor<T> {
        SerialEditor::new(self)
    }
}

#[derive(Debug)]
pub enum ExportError {
    SerializationError(ser::Error),
    IOError(io::Error),
}

impl From<io::Error> for ExportError {
    fn from(io_error: io::Error) -> Self {
        ExportError::IOError(io_error)
    }
}

impl From<ser::Error> for ExportError {
    fn from(serial_error: ser::Error) -> Self {
        ExportError::SerializationError(serial_error)
    }
}

impl<'t, T, E, ED> NodeEditor<'t, T, E, ED>
where
    T: Clone + Serialize,
{
    pub fn export_to_buf<W: io::Write>(&self, w: &mut W) -> Result<(), ExportError> {
        let serializable = self.export();
        let serialized = ser::to_string(&serializable)?;
        w.write(serialized.as_bytes())?;
        Ok(w.flush()?)
    }

    pub fn export_to_file<P: AsRef<Path>>(&self, file_path: P) -> Result<(), ExportError> {
        let mut f = fs::File::create(file_path)?;
        self.export_to_buf(&mut f)
    }
}

impl<'t, T, E, ED> NodeEditor<'t, T, E, ED>
where
    T: 'static + Clone + NamedAlgorithms<E> + VariantName,
    E: 'static,
{
    pub fn import(&mut self, import: DeserEditor<T, E>) -> Result<(), ImportError<E>> {
        self.dst = import.dst.into()?;
        self.node_states = {
            let mut node_states = NodeStates::new();
            for (node_id, state) in import.node_states {
                unsafe {
                    node_states.insert(node_id, state);
                }
            }
            node_states
        };
        // Reset cache
        self.output_results = {
            let mut output_results = BTreeMap::new();
            for (output_id, _) in self.dst.outputs_iter() {
                output_results.insert(*output_id, compute::new_compute_result());
            }
            output_results
        };
        Ok(())
    }
}
