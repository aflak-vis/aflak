use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;
use std::slice;
use std::sync::RwLock;

use boow::Bow;

use dst::node::{Node, NodeId};
use dst::{DSTError, Input, InputList, Output, OutputId, TransformIdx, DST};
use transform::Transformation;

impl<'t, T: 't, E: 't> DST<'t, T, E>
where
    T: Clone,
{
    /// Make a new empty [`DST`].
    pub fn new() -> Self {
        Self {
            transforms: HashMap::new(),
            edges: HashMap::new(),
            outputs: HashMap::new(),
            cache: HashMap::new(),
        }
    }

    /// Get a transform from its [`TransformIdx`].
    pub fn get_transform(&self, idx: &TransformIdx) -> Option<&Transformation<T, E>> {
        self.transforms.get(idx).map(|t| t.borrow())
    }

    /// Get a transform mutably from its [`TransformIdx`].
    /// Return `None` if the target transform is not owned.
    pub fn get_transform_mut(&mut self, idx: &TransformIdx) -> Option<&mut Transformation<T, E>> {
        self.transforms.get_mut(idx).and_then(|t| t.borrow_mut())
    }

    /// Get a node from its [`NodeId`].
    pub fn get_node(&self, idx: &NodeId) -> Option<Node<T, E>> {
        match idx {
            &NodeId::Transform(ref t_idx) => self.get_transform(t_idx).map(Node::Transform),
            &NodeId::Output(ref output_id) => self
                .outputs
                .get(output_id)
                .map(|some_output| Node::Output(some_output.as_ref())),
        }
    }

    /// Get a transform's dependencies, i.e the outputs wired into the transform's inputs, from its
    /// TransformIdx.
    /// The dependencies are ordered by InputIdx. Contains None if argument is currently not
    /// provided in the graph, Some(Output) otherwise.
    pub(crate) fn get_transform_dependencies(&self, idx: &TransformIdx) -> Vec<Option<Output>> {
        let t = self
            .get_transform(idx)
            .expect(&format!("Transform not found {:?}", idx));
        let len = t.input.len();
        (0..len)
            .map(|i| self.find_output_attached_to(&Input::new(*idx, i)))
            .collect()
    }

    fn find_output_attached_to(&self, input: &Input) -> Option<Output> {
        for (output, inputs) in self.edges.iter() {
            if inputs.contains(input) {
                return Some(*output);
            }
        }
        None
    }

    pub(crate) fn inputs_attached_to(&self, output: &Output) -> Option<slice::Iter<Input>> {
        self.edges
            .get(output)
            .map(|input_list| input_list.inputs.iter())
    }

    pub(crate) fn outputs_of_transformation(&self, t_idx: &TransformIdx) -> Option<Vec<Output>> {
        self.get_transform(&t_idx).map(|t| {
            let mut outputs = Vec::with_capacity(t.output.len());
            for i in 0..(t.output.len()) {
                let output = Output::new(*t_idx, i);
                if self.edges.contains_key(&output) {
                    outputs.push(output)
                } else if self
                    .outputs
                    .values()
                    .find(|&val| &Some(output) == val)
                    .is_some()
                {
                    outputs.push(output)
                }
            }
            outputs
        })
    }

    /// Add a borrowed transform and return its identifier [`TransformIdx`].
    pub fn add_transform(&mut self, t: &'t Transformation<T, E>) -> TransformIdx {
        let idx = self.new_transform_idx();
        self.transforms.insert(idx, Bow::Borrowed(t));
        idx
    }

    /// Create transform with the [`TransformIdx`] of your choosing.
    ///
    /// You need to manage your resource yourself so take care.
    /// Use [`DST::add_transform`] to have aflak manages resources for you
    /// (that's probably what your want).
    pub(crate) unsafe fn add_transform_with_idx(
        &mut self,
        idx: TransformIdx,
        t: Bow<'t, Transformation<T, E>>,
    ) {
        self.transforms.insert(idx, t);
    }

    /// Add an owned transform and return its identifier [`TransformIdx`].
    pub fn add_owned_transform(&mut self, t: Transformation<T, E>) -> TransformIdx {
        let idx = self.new_transform_idx();
        self.transforms.insert(idx, Bow::Owned(t));
        idx
    }

    /// Connect an output to an input.
    /// Returns an error if cycle is created or if output or input does not exist.
    ///
    /// If input is already connector to another output, delete this output.
    pub fn connect(&mut self, output: Output, input: Input) -> Result<(), DSTError<E>> {
        if !self.output_exists(&output) {
            Err(DSTError::InvalidOutput(format!(
                "{:?} does not exist in this graph!",
                output
            )))
        } else if !self.input_exists(&input) {
            Err(DSTError::InvalidInput(format!(
                "{:?} does not exist in this graph!",
                input
            )))
        } else if self.edge_exists(&input, &output) {
            Err(DSTError::DuplicateEdge(format!(
                "There already is an edge connecting {:?} to {:?}!",
                output, input
            )))
        } else if self.will_be_cycle(&input, &output) {
            Err(DSTError::Cycle(format!(
                "Connecting {:?} to {:?} would create a cycle!",
                output, input
            )))
        } else if !self.is_edge_compatible(&input, &output) {
            Err(DSTError::IncompatibleTypes(format!(
                "Cannot connect {:?} to {:?}. Output does not provided the required input type.",
                output, input
            )))
        } else {
            // Delete input if it is already attached somewhere
            for input_list in self.edges.values_mut() {
                input_list.inputs.retain(|input_| input_ != &input)
            }
            if !self.edges.contains_key(&output) {
                self.edges.insert(output, InputList::new(vec![input]));
            } else {
                let inputs = self.edges.get_mut(&output).unwrap();
                inputs.push(input);
            }
            self.purge_cache(output);
            Ok(())
        }
    }

    /// Attach an output to the graph. Only the attached outputs are lazily evaluated.
    /// Return the unique identifier to the attached output.
    /// Return an error if specified output does not exists in current graph.
    pub fn attach_output(&mut self, output: Output) -> Result<OutputId, DSTError<E>> {
        if self.output_exists(&output) {
            let idx = self.new_output_id();
            self.update_output(idx, output);
            Ok(idx)
        } else {
            Err(DSTError::InvalidOutput(format!(
                "{:?} does not exist in this graph!",
                output
            )))
        }
    }

    /// Create a new output not attached and return its Id.
    pub fn create_output(&mut self) -> OutputId {
        let idx = self.new_output_id();
        self.outputs.insert(idx, None);
        idx
    }

    /// Create output with the [`OutputId`] of your choosing.
    ///
    /// You need to manage your resource yourself so take care.
    /// Use [`DST::create_output`] to have aflak manages resources for you
    /// (that's probably what your want).
    pub(crate) unsafe fn create_output_with_id(&mut self, output_id: OutputId) {
        self.outputs.insert(output_id, None);
    }

    /// Attach an already registered output somewhere else
    pub fn update_output(&mut self, output_id: OutputId, output: Output) {
        self.outputs.insert(output_id, Some(output));
        self.cache.insert(output, RwLock::new(None));
    }

    /// Detach output with given ID. Does nothing if output does not exist.
    pub fn detach_output<O>(&mut self, output_id: &O)
    where
        OutputId: Borrow<O>,
        O: Hash + Eq,
    {
        self.outputs.remove(output_id);
    }

    /// Check that input exists in the current graph
    fn input_exists(&self, input: &Input) -> bool {
        match self.transforms.get(&input.t_idx) {
            None => false,
            Some(transform) => transform.input_exists(input.input_i.into()),
        }
    }

    /// Check that output exists in the current graph
    fn output_exists(&self, output: &Output) -> bool {
        match self.transforms.get(&output.t_idx) {
            None => false,
            Some(transform) => transform.output_exists(output.output_i.into()),
        }
    }

    /// Check that the edge exists in the current graph
    fn edge_exists(&self, input: &Input, output: &Output) -> bool {
        self.edges
            .get(&output)
            .map(|input_list| input_list.contains(input))
            .unwrap_or(false)
    }

    /// Check if a cycle will be created if the input and output given in argument are connected.
    ///
    /// Make dependency list for *output*'s transform and check that it does not depend on
    /// *input*'s transform.
    /// Assume input and output exists and that no cycle already exist in the data.
    fn will_be_cycle(&self, input: &Input, output: &Output) -> bool {
        for dep in self._dependencies(*output) {
            if dep.transform_idx() == input.t_idx {
                return true;
            }
        }
        false
    }

    /// Check if edge can be added to the current graph.
    /// Especially check if the input type is the same as the output type.
    fn is_edge_compatible(&self, input: &Input, output: &Output) -> bool {
        match (
            self.get_transform(&input.t_idx),
            self.get_transform(&output.t_idx),
        ) {
            (Some(input_t), Some(output_t)) => {
                input_t.nth_input_type(input.input_i.into())
                    == output_t.nth_output_type(output.output_i.into())
            }
            _ => false,
        }
    }

    fn new_transform_idx(&self) -> TransformIdx {
        self.transforms
            .keys()
            .max()
            .unwrap_or(&TransformIdx(0))
            .incr()
    }

    fn new_output_id(&self) -> OutputId {
        self.outputs.keys().max().unwrap_or(&OutputId(0)).incr()
    }
}
