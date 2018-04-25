use std::collections::BTreeMap;

use cake::{DeserDST, ImportError, NamedAlgorithms, NodeId, SerialDST, VariantName};

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
