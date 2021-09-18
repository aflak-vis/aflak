use crate::dst::{Output, OutputId, TransformIdx};
use crate::transform::{Transform, TypeId};
use variant_name::VariantName;

/// Identifies a [`Node`] in a [`DST`]. A node can either be a [`Transform`],
/// in that case it is identified by a [`TransformIdx`], or an [`OutputId`].
///
/// Use it together with [`DST::get_node`].
#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum NodeId {
    Transform(TransformIdx),
    Output(OutputId),
}

/// Represents a [`Node`], which is either a [`Transform`] or some
/// [`Output`].
pub enum Node<'a, 't: 'a, T: 't, E: 't> {
    Transform(&'a Transform<'t, T, E>),
    /// [`Output`] is `None` when there is an [`OutputId`] not connected to any
    /// [`Output`].
    Output((Option<&'a Output>, String)),
}

impl<'a, 't, T, E> Node<'a, 't, T, E>
where
    T: Clone + VariantName,
{
    /// Iterate over each default value
    pub fn inputs_default_iter(&self) -> Vec<Option<T>> {
        match *self {
            Node::Transform(t) => t.defaults(),
            Node::Output(_) => vec![None],
        }
    }
}

impl<'a, 't, T: VariantName, E> Node<'a, 't, T, E> {
    /// Get node's name.
    pub fn name(&self, id: &NodeId) -> String {
        match (self, id) {
            (Node::Transform(t), NodeId::Transform(t_idx)) => {
                format!("#{} {}", t_idx.id(), t.name())
            }
            (Node::Output((_, name)), NodeId::Output(output_id)) => {
                format!("Output #{} {}", output_id.id(), name.clone())
            }
            (Node::Transform(_), node_id) => panic!(
                "Node and NodeId do not agree on their types. Got a Node::Transform with Id {:?}.",
                node_id
            ),
            (Node::Output(ouput_id), node_id) => panic!(
                "Node and NodeId do not agree on their types. Got Node::Output({:?}) with Id {:?}.",
                ouput_id, node_id
            ),
        }
    }

    /// Iterate over name of each input slot
    pub fn input_slot_names_iter(&self) -> Vec<String>
    where
        T: Clone,
    {
        match *self {
            Node::Transform(t) => t.inputs().iter().map(|s| s.name_with_type()).collect(),
            Node::Output(_) => vec!["Out".to_owned()],
        }
    }

    /// Return number of inputs
    pub fn inputs_count(&self) -> usize {
        match *self {
            Node::Transform(t) => t.input_types().len(),
            Node::Output(_) => 1,
        }
    }

    /// Iterate over each type of the outputs
    pub fn outputs_iter(&self) -> Vec<TypeId> {
        match *self {
            Node::Transform(t) => t.outputs(),
            Node::Output(_) => vec![],
        }
    }

    /// Return number of outputs
    pub fn outputs_count(&self) -> usize {
        match *self {
            Node::Transform(t) => t.outputs().len(),
            Node::Output(_) => 0,
        }
    }
}
