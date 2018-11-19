use std::borrow::Cow;

use dst::{Output, OutputId, TransformIdx};
use transform::{Transform, TypeId};
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
pub enum Node<'a, T: 'a + Clone, E: 'a> {
    Transform(&'a Transform<T, E>),
    /// [`Output`] is `None` when there is an [`OutputId`] not connected to any
    /// [`Output`].
    Output(Option<&'a Output>),
}

impl<'a, T: Clone + VariantName, E> Node<'a, T, E> {
    /// Get node's name.
    pub fn name(&'a self, id: &NodeId) -> Cow<'static, str> {
        match *self {
            Node::Transform(t) => Cow::Borrowed(t.name()),
            Node::Output(_) => {
                if let NodeId::Output(output_id) = id {
                    let OutputId(id) = output_id;
                    Cow::Owned(format!("Output #{}", id))
                } else {
                    panic!("Expected id to be output")
                }
            }
        }
    }

    /// Iterate over name of each input slot
    pub fn input_slot_names_iter(&self) -> Vec<&'static str> {
        match *self {
            Node::Transform(t) => t.inputs().into_iter().map(|s| s.name()).collect(),
            Node::Output(_) => vec!["Out"],
        }
    }

    /// Iterate over each default value
    pub fn inputs_default_iter(&self) -> Vec<Option<T>> {
        match *self {
            Node::Transform(t) => t.defaults(),
            Node::Output(_) => vec![None],
        }
    }

    /// Return number of inputs
    pub fn inputs_count(&self) -> usize {
        match *self {
            Node::Transform(t) => t.inputs().len(),
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
