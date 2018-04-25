use cake::{NodeId, SerialDST};

use editor::NodeEditor;
use node_state::NodeState;

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

impl<'t, T, E, ED> NodeEditor<'t, T, E, ED>
where
    T: Clone,
{
    pub fn export(&self) -> SerialEditor<T> {
        SerialEditor::new(self)
    }
}
