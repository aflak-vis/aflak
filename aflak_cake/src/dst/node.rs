use std::borrow::Cow;
use std::slice;

use dst::{Output, OutputId, TransformIdx};
use transform::Transformation;

#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug)]
pub enum NodeId {
    Transform(TransformIdx),
    Output(OutputId),
}

pub enum Node<'a, T: 'a + Clone, E: 'a> {
    Transform(&'a Transformation<T, E>),
    Output(Option<&'a Output>),
}

impl<'a, T: Clone, E> Node<'a, T, E> {
    pub fn name(&'a self, id: &NodeId) -> Cow<'static, str> {
        match self {
            &Node::Transform(t) => Cow::Borrowed(t.name),
            &Node::Output(_) => {
                if let NodeId::Output(output_id) = id {
                    Cow::Owned(format!("Output {:?}", output_id))
                } else {
                    panic!("Expected id to be output")
                }
            }
        }
    }

    pub fn inputs_iter(&'a self) -> slice::Iter<'a, &'static str> {
        const OUTPUT_NODE_SLOTS: [&'static str; 1] = ["Out"];
        match self {
            &Node::Transform(t) => t.input.iter(),
            &Node::Output(_) => OUTPUT_NODE_SLOTS.iter(),
        }
    }

    pub fn inputs_count(&self) -> usize {
        match self {
            &Node::Transform(t) => t.input.len(),
            &Node::Output(_) => 1,
        }
    }

    pub fn outputs_iter(&'a self) -> slice::Iter<'a, &'static str> {
        const OUTPUT_NODE_SLOTS: [&'static str; 0] = [];
        match self {
            &Node::Transform(t) => t.output.iter(),
            &Node::Output(_) => OUTPUT_NODE_SLOTS.iter(),
        }
    }

    pub fn outputs_count(&self) -> usize {
        match self {
            &Node::Transform(t) => t.output.len(),
            &Node::Output(_) => 0,
        }
    }
}
