use boow::Bow;
pub extern crate uuid;

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::time::Instant;

use transform::{Algorithm, Transform};
use variant_name::VariantName;

mod build;
pub mod compute;
mod iterators;
mod node;
pub use self::iterators::{Dependency, LinkIter, NodeIter};
pub use self::node::{Node, NodeId};
use uuid::Uuid;

/// Dynamic Syntax Tree
///
/// Represent the node graph for the computing tasks to be done.
/// Each node is identified by a [`NodeId`].
/// A DST has two types of nodes, transformation and output nodes.
/// An output node is a leaf, it is the end of the journey of the data.
/// A transformation node wraps a [`Transform`] to takes input data and
/// compute output data out of it.
///
/// Each output node is identified by an [`OutputId`], while each transformation
/// node is identified by a [`TransformIdx`].
#[derive(Debug)]
pub struct DST<'t, T: 't, E: 't> {
    transforms: BTreeMap<TransformIdx, MetaTransform<'t, T, E>>,
    edges: BTreeMap<Output, InputList>,
    outputs: BTreeMap<OutputId, Option<Output>>,
}

impl<'t, T, E> Clone for DST<'t, T, E>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            transforms: self.transforms.clone(),
            edges: self.edges.clone(),
            outputs: self.outputs.clone(),
        }
    }
}

impl<'t, T, E> Default for DST<'t, T, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'t, T, E> DST<'t, T, E>
where
    T: VariantName,
{
    /// Get max(updated_on) for all this transform's dependencies.
    /// panic if TransformIdx does not exist
    pub fn updated_on(&self, t_idx: TransformIdx) -> Instant {
        let mut updated_on = self.transforms[&t_idx].updated_on();
        for dep in self._dependencies(Output::new(t_idx, 0)) {
            let dep_t_idx = dep.transform_idx();
            let dep_updated_on = self.transforms[&dep_t_idx].updated_on();
            updated_on = updated_on.max(dep_updated_on);
        }
        updated_on
    }
}

/// An owned or borrowed Transform to which meta-data is added.
///
/// Meta-data includes default values when no node is connected to input, for
/// example.
#[derive(Debug)]
pub struct MetaTransform<'t, T: 't, E: 't> {
    t: Bow<'t, Transform<'t, T, E>>,
    input_defaults: Vec<Option<T>>,
    updated_on: Instant,
}

impl<'t, T, E> Clone for MetaTransform<'t, T, E>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            t: self.t.clone(),
            input_defaults: self.input_defaults.clone(),
            updated_on: self.updated_on,
        }
    }
}

impl<'t, T, E> MetaTransform<'t, T, E>
where
    T: Clone,
{
    pub fn new(t: Bow<'t, Transform<'t, T, E>>) -> Self {
        let input_defaults = if let Algorithm::Macro { .. } = t.algorithm() {
            vec![]
        } else {
            t.defaults()
        };
        Self {
            t,
            input_defaults,
            updated_on: Instant::now(),
        }
    }
}

impl<'t, T, E> MetaTransform<'t, T, E> {
    pub fn new_with_defaults(
        t: Bow<'t, Transform<'t, T, E>>,
        input_defaults: Vec<Option<T>>,
    ) -> Self {
        Self {
            t,
            input_defaults,
            updated_on: Instant::now(),
        }
    }

    pub fn transform(&self) -> &Transform<'t, T, E> {
        self.t.as_ref()
    }

    pub fn defaults(&self) -> ::std::borrow::Cow<'_, [Option<T>]>
    where
        T: Clone + VariantName,
    {
        if let Algorithm::Macro { handle } = self.t.algorithm() {
            let macro_defaults = handle.defaults();
            let mut defaults = Vec::with_capacity(macro_defaults.len());
            for (i, macro_default) in macro_defaults.into_iter().enumerate() {
                if let Some(some_input_default) = self.input_defaults.get(i) {
                    match (some_input_default.as_ref(), macro_default) {
                        (Some(input_default), Some(macro_default)) => {
                            if input_default.variant_name() == macro_default.variant_name() {
                                // Keep input default
                                defaults.push(Some(input_default.clone()));
                            } else {
                                defaults.push(Some(macro_default));
                            }
                        }
                        (_, some_macro_default) => defaults.push(some_macro_default),
                    }
                } else {
                    defaults.push(macro_default)
                }
            }
            ::std::borrow::Cow::Owned(defaults)
        } else {
            ::std::borrow::Cow::Borrowed(&self.input_defaults)
        }
    }

    pub fn transform_mut(&mut self) -> Option<&mut Transform<'t, T, E>> {
        self.t.borrow_mut()
    }

    pub fn defaults_mut(&mut self) -> InputDefaultsMut<'_, 't, T, E> {
        InputDefaultsMut { t: self }
    }

    pub fn tokenize(self) -> TransformAndDefaults<'t, T, E> {
        (self.t, self.input_defaults)
    }

    pub fn updated_on(&self) -> Instant {
        self.updated_on.max(self.t.updated_on())
    }

    pub(crate) fn updated_now(&mut self) {
        self.updated_on = Instant::now();
    }
}

pub struct InputDefaultsMut<'a, 't: 'a, T: 'a + 't, E: 'a + 't> {
    t: &'a mut MetaTransform<'t, T, E>,
}

impl<'a, 't, T, E> InputDefaultsMut<'a, 't, T, E> {
    pub fn write(&mut self, index: usize, value: T) {
        while self.t.input_defaults.len() <= index {
            self.t.input_defaults.push(None);
        }
        self.t.input_defaults[index] = Some(value);
        self.t.updated_on = Instant::now();
    }
}

/// Tuple of a transformation and the default input values set up for it
pub type TransformAndDefaults<'t, T, E> = (Bow<'t, Transform<'t, T, E>>, Vec<Option<T>>);

/// Uniquely identify an ouput of a transformation node
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Output {
    pub t_idx: TransformIdx,
    output_i: OutputIdx,
}

/// Uniquely identify an input of a node
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

impl fmt::Display for Output {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let output_no = self.output_i.0 + 1;
        write!(f, "output #{} of node #{}", output_no, self.t_idx.1)
    }
}

impl fmt::Display for Input {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "input #{} of node #{}", self.input_i.0 + 1, self.t_idx.1)
    }
}

#[derive(Debug, Clone)]
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
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TransformIdx(Option<Uuid>, usize);
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
struct OutputIdx(usize);
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
struct InputIdx(usize);
/// Identify an output node
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OutputId(usize);

/// Errors when computing or building a [`DST`].
#[derive(Debug)]
pub enum DSTError {
    InvalidInput(String),
    InvalidOutput(String),
    DuplicateEdge(String),
    Cycle(String),
    IncompatibleTypes(String),
}

impl fmt::Display for DSTError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use DSTError::*;

        match self {
            InvalidInput(s) => write!(f, "Invalid input! {}", s),
            InvalidOutput(s) => write!(f, "Invalid output! {}", s),
            DuplicateEdge(s) => write!(f, "Duplicated edge! {}", s),
            Cycle(s) => write!(f, "Cannot create cyclic dependency! {}", s),
            IncompatibleTypes(s) => write!(f, "Incompatible types! {}", s),
        }
    }
}

impl Error for DSTError {
    fn description(&self) -> &str {
        "aflak_cake::DSTError"
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
        TransformIdx(self.0, self.1 + 1)
    }
    pub fn macro_id(self) -> Option<Uuid> {
        self.0
    }
    pub fn id(self) -> usize {
        self.1
    }
    pub fn set_macro(self, macro_id: Uuid) -> Self {
        TransformIdx(Some(macro_id), self.1)
    }
}

impl OutputId {
    fn incr(self) -> Self {
        OutputId(self.0 + 1)
    }
    pub fn id(self) -> usize {
        self.0
    }
    pub fn new(id: usize) -> Self {
        OutputId(id)
    }
}

/// Identify an input slot, i.e., the input of a transform or the input of a
/// final output node.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize, Hash)]
pub enum InputSlot {
    Transform(Input),
    Output(OutputId),
}

/// Convenient implementation for debugging
impl<'t, T, E> fmt::Display for DST<'t, T, E>
where
    T: 't + VariantName,
    E: 't,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (output_id, output) in self.outputs_iter() {
            writeln!(f, "{:?}", output_id)?;
            if let Some(output) = output {
                self.write_output(f, 1, output)?;
            } else {
                writeln!(f, "{}*", pad(1))?;
            }
        }
        Ok(())
    }
}

impl<'t, T, E> DST<'t, T, E>
where
    T: 't + VariantName,
    E: 't,
{
    fn write_output(&self, f: &mut fmt::Formatter, depth: usize, output: &Output) -> fmt::Result {
        if let Some(meta) = self.transforms.get(&output.t_idx) {
            let t = meta.transform();
            writeln!(f, "{}{}", pad(depth), t.name())?;
            let deps = self.outputs_attached_to_transform(output.t_idx).unwrap();
            for (i, dep) in deps.into_iter().enumerate() {
                write!(f, "{}{}", pad(depth + 1), i)?;
                if let Some(dep) = dep {
                    writeln!(f)?;
                    self.write_output(f, depth + 2, &dep)?;
                } else {
                    writeln!(f, " (no node)")?;
                }
            }
            Ok(())
        } else {
            writeln!(f, "{}(missing node)", pad(depth))
        }
    }
}

fn pad(depth: usize) -> String {
    const SEPARATOR: &str = "\\_ ";
    const PADDER: &str = "    ";
    let mut out = String::with_capacity(depth * PADDER.len() + SEPARATOR.len());
    for _ in 0..depth {
        out.push_str(PADDER);
    }
    out.push_str(SEPARATOR);
    out
}
