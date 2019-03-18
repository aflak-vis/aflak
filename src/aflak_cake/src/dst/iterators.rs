use std::collections::btree_map;
use std::ops;
use std::slice;
use std::vec;

use dst::node::{Node, NodeId};
use dst::MetaTransform;
use dst::{Input, InputList, InputSlot, Output, OutputId, TransformIdx, DST};
use transform::Transform;
use variant_name::VariantName;

impl<'t, T: 't, E: 't> DST<'t, T, E> {
    pub(crate) fn transforms_iter(&self) -> TransformIterator<'_, 't, T, E> {
        TransformIterator::new(self.transforms.iter())
    }

    pub(crate) fn meta_transforms_iter(
        &self,
    ) -> btree_map::Iter<TransformIdx, MetaTransform<'t, T, E>> {
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
    pub fn outputs_iter(&self) -> btree_map::Iter<OutputId, Option<Output>> {
        self.outputs.iter()
    }

    /// Iterator over nodes.
    pub fn nodes_iter(&self) -> NodeIter<'_, 't, T, E> {
        NodeIter {
            transforms: self.transforms_iter(),
            outputs: self.outputs_iter(),
        }
    }

    /// Return owned vector containing all [`NodeId`]s.
    pub fn node_ids(&self) -> Vec<NodeId> {
        self.nodes_iter().map(|(id, _)| id).collect()
    }

    pub(crate) fn _dependencies(&self, output: Output) -> DependencyIter<'_, 't, T, E> {
        DependencyIter {
            dst: self,
            stack: vec![output],
            completed_stack: vec![],
        }
    }
}

/// Make a post-order tree traversal to look for deepest dependencies first.
/// Return the dependencies one at a time
pub struct DependencyIter<'a, 't: 'a, T: 't, E: 't> {
    dst: &'a DST<'t, T, E>,
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

impl<'a, 't, T: 't, E> Iterator for DependencyIter<'a, 't, T, E>
where
    T: VariantName,
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
    edges: btree_map::Iter<'a, Output, InputList>,
    output: Option<&'a Output>,
    inputs: slice::Iter<'a, Input>,
}

impl<'a> EdgeIterator<'a> {
    fn new(edges: btree_map::Iter<'a, Output, InputList>) -> Self {
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

pub struct TransformIterator<'a, 't: 'a, T: 't, E: 't> {
    iter: btree_map::Iter<'a, TransformIdx, MetaTransform<'t, T, E>>,
}
impl<'a, 't, T, E> TransformIterator<'a, 't, T, E> {
    fn new(iter: btree_map::Iter<'a, TransformIdx, MetaTransform<'t, T, E>>) -> Self {
        Self { iter }
    }
}

impl<'a, 't, T, E> Iterator for TransformIterator<'a, 't, T, E> {
    type Item = (&'a TransformIdx, &'a Transform<'t, T, E>);
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(idx, t)| (idx, t.transform()))
    }
}

/// Iterator over nodes.
///
/// Iterate over a tuple ([`NodeId`], [`Node`]).
pub struct NodeIter<'a, 't: 'a, T: 't, E: 't> {
    transforms: TransformIterator<'a, 't, T, E>,
    outputs: btree_map::Iter<'a, OutputId, Option<Output>>,
}

/// Iterate over nodes.
impl<'a, 't, T, E> Iterator for NodeIter<'a, 't, T, E> {
    type Item = (NodeId, Node<'a, 't, T, E>);
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
/// The input slot is either the input to another [`Transform`] or the
/// input slot of an output node.
pub struct LinkIter<'a> {
    edges: EdgeIterator<'a>,
    outputs: btree_map::Iter<'a, OutputId, Option<Output>>,
}

impl<'a> LinkIter<'a> {
    fn new(
        edges: EdgeIterator<'a>,
        outputs: btree_map::Iter<'a, OutputId, Option<Output>>,
    ) -> Self {
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

impl<'t, T, E> DST<'t, T, E>
where
    T: Clone,
{
    pub(crate) fn unattached_input_slots(&self) -> impl Iterator<Item = InputSlot> + '_ {
        let output_ids = self.unattached_output_ids().map(InputSlot::Output);
        let inputs = self
            .unattached_inputs()
            .into_iter()
            .map(InputSlot::Transform);
        output_ids.chain(inputs)
    }

    fn unattached_output_ids(&self) -> impl Iterator<Item = OutputId> + '_ {
        self.outputs
            .iter()
            .filter(|(_, some_output)| some_output.is_none())
            .map(|(output_id, _)| *output_id)
    }

    fn unattached_inputs(&self) -> Vec<Input> {
        self.transform_inputs()
            .filter(|input| {
                for (_, input_list) in &self.edges {
                    if input_list.contains(input) {
                        return false;
                    }
                }
                true
            })
            .collect()
    }

    fn transform_inputs(&self) -> impl Iterator<Item = Input> + '_ {
        TransformInputIterator::new(self)
    }
}

struct TransformInputIterator {
    inner: vec::IntoIter<(TransformIdx, usize)>,
    cursor: Option<TransformInputCursor>,
}

struct TransformInputCursor {
    t_idx: TransformIdx,
    inputs: ops::Range<usize>,
}

impl TransformInputCursor {
    fn next(&mut self) -> Option<Input> {
        self.inputs
            .next()
            .map(|index| Input::new(self.t_idx, index))
    }
}

impl TransformInputIterator {
    fn new<T, E>(dst: &DST<'_, T, E>) -> Self
    where
        T: Clone,
    {
        Self {
            inner: dst
                .transforms
                .iter()
                .map(|(t_idx, meta)| (*t_idx, meta.transform().inputs().len()))
                .collect::<Vec<_>>()
                .into_iter(),
            cursor: None,
        }
    }
}

impl Iterator for TransformInputIterator {
    type Item = Input;
    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor.is_none() {
            if let Some((t_idx, len)) = self.inner.next() {
                self.cursor = Some(TransformInputCursor {
                    t_idx,
                    inputs: 0..len,
                });
            } else {
                return None;
            }
        }

        let mut exhausted = false;
        if let Some(ref mut cursor) = &mut self.cursor {
            if let Some(input) = cursor.next() {
                return Some(input);
            } else {
                exhausted = true;
            }
        }

        if exhausted {
            self.cursor = None;
        }
        self.next()
    }
}
