use std::borrow::Borrow;
use std::hash::Hash;
use std::collections::HashMap;

use transform::{Transformation, TypeContent};

pub struct DST<'de, T: 'de + TypeContent> {
    transforms: HashMap<TransformIdx, &'de Transformation<'de, T>>,
    edges: HashMap<Output, InputList<T>>,
    outputs: HashMap<OutputId, Output>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Output {
    t_idx: TransformIdx,
    ouput_i: OutputIdx,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Input {
    t_idx: TransformIdx,
    input_i: InputIdx,
}

struct InputList<T> {
    /// List of all inputs to which the data is fed
    inputs: Vec<Input>,
    /// Compute cache
    cache: Option<T>,
}

impl<T> InputList<T> {
    pub fn new(inputs: Vec<Input>) -> Self {
        Self {
            inputs,
            cache: None,
        }
    }

    pub fn push(&mut self, input: Input) {
        self.inputs.push(input);
    }

    pub fn contains(&self, input: &Input) -> bool {
        self.inputs.contains(input)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransformIdx(usize);
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct OutputIdx(usize);
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct InputIdx(usize);
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OutputId(usize);

pub enum DSTError {
    InvalidInput(String),
    InvalidOutput(String),
    DuplicateEdge(String),
    Cycle(String),
}

impl<'de, T: TypeContent> DST<'de, T> {
    pub fn new() -> Self {
        Self {
            transforms: HashMap::new(),
            edges: HashMap::new(),
            outputs: HashMap::new(),
        }
    }

    pub fn contains(&self, t: &'de Transformation<'de, T>) -> bool {
        let ptr = t as *const Transformation<T>;
        for transform in self.transforms.values() {
            if *transform as *const Transformation<T> == ptr {
                return true;
            }
        }
        false
    }

    /// Add a transform and return its identifier TransformIdx.
    pub fn add_transform(&mut self, t: &'de Transformation<'de, T>) -> TransformIdx {
        let idx = self.new_transform_idx();
        self.transforms.insert(idx, t);
        idx
    }

    /// Connect an output to an input.
    /// Returns an error if cycle is created or if output or input does not exist.
    pub fn connect(&mut self, output: Output, input: Input) -> Result<(), DSTError> {
        if !self.check_output(&output) {
            Err(DSTError::InvalidOutput(format!("{:?} does not exist in this graph!", output)))
        } else if !self.check_input(&input) {
            Err(DSTError::InvalidInput(format!("{:?} does not exist in this graph!", input)))
        } else if !self.edge_exists(&input, &output) {
            Err(DSTError::DuplicateEdge(format!("There already is an edge connecting {:?} to {:?}!", output, input)))
        } else if !self.check_cycle(&input, &output) {
            Err(DSTError::Cycle(format!("Connecting {:?} to {:?} would create a cycle!", output, input)))
        } else {
            if !self.edges.contains_key(&output) {
                self.edges.insert(output, InputList::new(vec![input]));
            } else {
                let inputs = self.edges.get_mut(&output).unwrap();
                inputs.push(input);
            }
            Ok(())
        }
    }

    /// Attach an output to the graph. Only the attached outputs are lazily evaluated.
    /// Return the unique identifier to the attached output.
    /// Return an error if specified output does not exists in current graph.
    pub fn attach_output(&mut self, output: Output) -> Result<OutputId, DSTError> {
        if self.check_output(&output) {
            let idx = self.new_output_id();
            self.outputs.insert(idx, output);
            Ok(idx)
        } else {
            Err(DSTError::InvalidOutput(format!("{:?} does not exist in this graph!", output)))
        }
    }

    /// Detach output with given ID. Does nothing if output does not exist.
    pub fn detach_output<O>(&mut self, output_id: &O)
        where OutputId: Borrow<O>,
              O: Hash + Eq,
    {
        self.outputs.remove(output_id);
    }

    /// Check that input exists in the current graph
    fn check_input(&self, input: &Input) -> bool {
        match self.transforms.get(&input.t_idx) {
            None => false,
            Some(transform) => transform.check_input(input.input_i.into())
        }
    }

    /// Check that output exists in the current graph
    fn check_output(&self, output: &Output) -> bool {
        match self.transforms.get(&output.t_idx) {
            None => false,
            Some(transform) => transform.check_output(output.ouput_i.into())
        }
    }

    fn edge_exists(&self, input: &Input, output: &Output) -> bool {
        self.edges.get(&output).map(|input_list| {
            input_list.contains(input)
        }).unwrap_or(false)
    }

    /// Check if a cycle will be created if the input and output given in argument are connected.
    ///
    /// Make dependency list for *output*'s transform and check that it does not depend on
    /// *input*'s transform.
    fn check_cycle(&self, input: &Input, output: &Output) -> bool {
        unimplemented!()
    }

    fn new_transform_idx(&self) -> TransformIdx {
        self.transforms.keys().max().unwrap_or(&TransformIdx(0)).incr()
    }

    fn new_output_id(&self) -> OutputId {
        self.outputs.keys().max().unwrap_or(&OutputId(0)).incr()
    }
}


impl From<OutputIdx> for usize {
    fn from(output: OutputIdx) -> usize {
        output.0
    }
}
impl From<InputIdx> for usize {
    fn from(input: InputIdx) -> usize {
        input.0
    }
}

impl TransformIdx {
    fn incr(self) -> Self {
        TransformIdx(self.0 + 1)
    }
}

impl OutputId {
    fn incr(self) -> Self {
        OutputId(self.0 + 1)
    }
}
