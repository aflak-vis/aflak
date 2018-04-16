use boow::Bow;

use std::borrow::Cow;
use std::collections::HashMap;
use std::slice;
use std::sync::RwLock;

use transform::Transformation;

mod build;
mod compute;
mod iterators;
mod node;

type Cache<T> = RwLock<Option<T>>;

/// Dynamic Syntax Tree
///
/// Represent the node graph for the computing tasks to be done.
/// Each node is identified by a [`NodeId`].
/// A DST has two types of nodes, transformation and output nodes.
/// An output node is a leaf, it is the end of the journey of the data.
/// A transformation node wraps a [`Transformation`] to takes input data and
/// compute output data out of it.
///
/// Each output node is identified by an [`OutputId`], while each transformation
/// node is identified by a [`TransformIdx`].
#[derive(Debug)]
pub struct DST<'t, T: Clone + 't, E: 't> {
    transforms: HashMap<TransformIdx, Bow<'t, Transformation<T, E>>>,
    edges: HashMap<Output, InputList>,
    outputs: HashMap<OutputId, Option<Output>>,
    cache: HashMap<Output, Cache<T>>,
}

/// Uniquely identify an ouput of a transformation node
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Output {
    pub t_idx: TransformIdx,
    output_i: OutputIdx,
}

/// Uniquely identify an input of a node
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Input {
    pub t_idx: TransformIdx,
    input_i: InputIdx,
}

impl Output {
    /// Create a new Output pointing to the *out_i*-th output of TransformIdx transform.
    /// Counting start from 0.
    pub fn new(t_idx: TransformIdx, out_i: usize) -> Self {
        Self {
            t_idx,
            output_i: OutputIdx(out_i),
        }
    }
    /// Get index of output (starting from 0 for the first output).
    pub fn index(&self) -> usize {
        self.output_i.into()
    }
}

impl Input {
    /// Create a new Input pointing to the *in_i*-th input of TransformIdx transform.
    /// Counting start from 0.
    pub fn new(t_idx: TransformIdx, in_i: usize) -> Self {
        Self {
            t_idx,
            input_i: InputIdx(in_i),
        }
    }
    /// Get index of input (starting from 0 for the first input).
    pub fn index(&self) -> usize {
        self.input_i.into()
    }
}

#[derive(Debug)]
struct InputList {
    /// List of all inputs to which the data is fed
    inputs: Vec<Input>,
}

impl InputList {
    pub fn new(inputs: Vec<Input>) -> Self {
        Self { inputs }
    }

    pub fn push(&mut self, input: Input) {
        self.inputs.push(input);
    }

    pub fn contains(&self, input: &Input) -> bool {
        self.inputs.contains(input)
    }
}

/// Identify a transformation node
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransformIdx(usize);
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct OutputIdx(usize);
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct InputIdx(usize);
/// Identify an output node
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OutputId(usize);

/// Errors when computing or building a [`DST`].
#[derive(Debug)]
pub enum DSTError<E> {
    InvalidInput(String),
    InvalidOutput(String),
    DuplicateEdge(String),
    Cycle(String),
    IncompatibleTypes(String),
    MissingOutputID(String),
    ComputeError(String),
    InnerComputeError(E),
    NothingDoneYet,
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
    pub fn id(&self) -> usize {
        self.0
    }
}

impl OutputId {
    fn incr(self) -> Self {
        OutputId(self.0 + 1)
    }
}

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

#[derive(Copy, Clone)]
pub enum InputSlot<'a> {
    Transform(&'a Input),
    Output(&'a OutputId),
}
