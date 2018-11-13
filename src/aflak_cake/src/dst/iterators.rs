use std::collections::hash_map;
use std::slice;

use dst::node::{Node, NodeId};
use dst::MetaTransform;
use dst::{Input, InputList, InputSlot, Output, OutputId, TransformIdx, DST};
use transform::Transformation;

impl<'t, T: 't, E: 't> DST<'t, T, E>
where
    T: Clone,
{
    pub(crate) fn transforms_iter(&self) -> TransformIterator<T, E> {
        TransformIterator::new(self.transforms.iter())
    }

    pub(crate) fn meta_transforms_iter(&self) -> hash_map::Iter<TransformIdx, MetaTransform<T, E>> {
        self.transforms.iter()
    }

    pub(crate) fn edges_iter(&self) -> EdgeIterator {
        EdgeIterator::new(self.edges.iter())
    }

    /// Iterator over links.
    pub fn links_iter(&self) -> LinkIter {
        LinkIter::new(self.edges_iter(), self.outputs_iter())
    }

    /// Iterator over outputs.
    pub fn outputs_iter(&self) -> hash_map::Iter<OutputId, Option<Output>> {
        self.outputs.iter()
    }

    /// Iterator over nodes.
    pub fn nodes_iter(&self) -> NodeIter<T, E> {
        NodeIter {
            transforms: self.transforms_iter(),
            outputs: self.outputs_iter(),
        }
    }

    /// Return owned vector containing all [`NodeId`]s.
    pub fn node_ids(&self) -> Vec<NodeId> {
        self.nodes_iter().map(|(id, _)| id).collect()
    }

    pub(crate) fn _dependencies(&'t self, output: Output) -> DependencyIter<'t, T, E> {
        DependencyIter {
            dst: self,
            stack: vec![output],
            completed_stack: vec![],
        }
    }
}

/// Make a post-order tree traversal to look for deepest dependencies first.
/// Return the dependencies one at a time
pub struct DependencyIter<'t, T: 't + Clone, E: 't> {
    dst: &'t DST<'t, T, E>,
    stack: Vec<Output>,
    completed_stack: Vec<Dependency>,
}

pub struct Dependency {
    t_idx: TransformIdx,
}

impl Dependency {
    pub fn transform_idx(&self) -> TransformIdx {
        self.t_idx
    }
}

impl<'t, T: 't, E> Iterator for DependencyIter<'t, T, E>
where
    T: Clone,
{
    type Item = Dependency;
    /// Push all parents on the stack recursively.
    /// If value has no parents, pop the stack and return it.
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(current_output) = self.stack.pop() {
            if let Some(mut parent_outputs) =
                self.dst.outputs_attached_to_transform(current_output.t_idx)
            {
                let dep = Dependency {
                    t_idx: current_output.t_idx,
                };
                if parent_outputs.is_empty() {
                    Some(dep)
                } else {
                    parent_outputs.retain(Option::is_some);
                    self.stack.extend(
                        parent_outputs
                            .into_iter()
                            .map(Option::unwrap)
                            .collect::<Vec<_>>(),
                    );
                    self.completed_stack.push(dep);
                    self.next()
                }
            } else {
                self.completed_stack.pop()
            }
        } else {
            self.completed_stack.pop()
        }
    }
}

pub struct EdgeIterator<'a> {
    edges: hash_map::Iter<'a, Output, InputList>,
    output: Option<&'a Output>,
    inputs: slice::Iter<'a, Input>,
}

impl<'a> EdgeIterator<'a> {
    fn new(edges: hash_map::Iter<'a, Output, InputList>) -> Self {
        const NO_INPUT: [Input; 0] = [];
        Self {
            edges,
            output: None,
            inputs: NO_INPUT.iter(),
        }
    }
}

impl<'a> Iterator for EdgeIterator<'a> {
    type Item = (&'a Output, &'a Input);
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(input) = self.inputs.next() {
            Some((self.output.unwrap(), input))
        } else if let Some((output, input_list)) = self.edges.next() {
            self.output = Some(output);
            self.inputs = input_list.inputs.iter();
            self.next()
        } else {
            None
        }
    }
}

pub struct TransformIterator<'a, T: 'a + Clone, E: 'a> {
    iter: hash_map::Iter<'a, TransformIdx, MetaTransform<'a, T, E>>,
}
impl<'a, T: Clone, E> TransformIterator<'a, T, E> {
    fn new(iter: hash_map::Iter<'a, TransformIdx, MetaTransform<T, E>>) -> Self {
        Self { iter }
    }
}

impl<'a, T: Clone, E> Iterator for TransformIterator<'a, T, E> {
    type Item = (&'a TransformIdx, &'a Transformation<T, E>);
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(idx, t)| (idx, t.transform()))
    }
}

/// Iterator over nodes.
///
/// Iterate over a tuple ([`NodeId`], [`Node`]).
pub struct NodeIter<'a, T: 'a + Clone, E: 'a> {
    transforms: TransformIterator<'a, T, E>,
    outputs: hash_map::Iter<'a, OutputId, Option<Output>>,
}

/// Iterate over nodes.
impl<'a, T: Clone, E> Iterator for NodeIter<'a, T, E> {
    type Item = (NodeId, Node<'a, T, E>);
    fn next(&mut self) -> Option<Self::Item> {
        if let Some((id, t)) = self.transforms.next() {
            Some((NodeId::Transform(*id), Node::Transform(t)))
        } else if let Some((id, o)) = self.outputs.next() {
            Some((NodeId::Output(*id), Node::Output(o.as_ref())))
        } else {
            None
        }
    }
}

/// Iterator over links.
///
/// A link is a tuple ([`Output`], [`InputSlot`]). It is attached on one side to
/// the [`Output`] of a transformation and to the other side on an input slot.
/// The input slot is either the input to another [`Transformation`] or the
/// input slot of an output node.
pub struct LinkIter<'a> {
    edges: EdgeIterator<'a>,
    outputs: hash_map::Iter<'a, OutputId, Option<Output>>,
}

impl<'a> LinkIter<'a> {
    fn new(edges: EdgeIterator<'a>, outputs: hash_map::Iter<'a, OutputId, Option<Output>>) -> Self {
        Self { edges, outputs }
    }
}

/// Iterate over links.
impl<'a> Iterator for LinkIter<'a> {
    type Item = (&'a Output, InputSlot);
    fn next(&mut self) -> Option<Self::Item> {
        if let Some((output, input)) = self.edges.next() {
            Some((output, InputSlot::Transform(*input)))
        } else if let Some((output_id, output)) = self.outputs.next() {
            if let Some(output) = output {
                Some((output, InputSlot::Output(*output_id)))
            } else {
                self.next()
            }
        } else {
            None
        }
    }
}
