use variant_name::VariantName;
use std::borrow::Borrow;
use std::hash::Hash;
use std::collections::HashMap;

use transform::{NamedAlgorithms, Transformation};

#[derive(Serialize)]
pub struct DST<'t, T: Clone + 't, E: 't> {
    transforms: HashMap<TransformIdx, &'t Transformation<T, E>>,
    edges: HashMap<Output, InputList>,
    outputs: HashMap<OutputId, Output>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Output {
    t_idx: TransformIdx,
    output_i: OutputIdx,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Input {
    t_idx: TransformIdx,
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
}

#[derive(Serialize, Deserialize)]
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TransformIdx(usize);
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct OutputIdx(usize);
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct InputIdx(usize);
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OutputId(usize);

#[derive(Debug)]
pub enum DSTError {
    InvalidInput(String),
    InvalidOutput(String),
    DuplicateEdge(String),
    Cycle(String),
    IncompatibleTypes(String),
    MissingOutputID(String),
    ComputeError(String),
}

impl<'t, T: 't, E: 't> DST<'t, T, E>
where
    T: Clone + VariantName,
{
    pub fn new() -> Self {
        Self {
            transforms: HashMap::new(),
            edges: HashMap::new(),
            outputs: HashMap::new(),
        }
    }

    pub fn contains(&self, t: &'t Transformation<T, E>) -> bool {
        let ptr = t as *const Transformation<T, E>;
        for transform in self.transforms.values() {
            if *transform as *const Transformation<T, E> == ptr {
                return true;
            }
        }
        false
    }

    /// Get a transform from its TransformIdx.
    pub fn get_transform(&self, idx: &TransformIdx) -> Option<&'t Transformation<T, E>> {
        self.transforms.get(idx).map(|t| *t)
    }

    /// Get a transform's dependencies, i.e the outputs wired into the transform's inputs, from its
    /// TransformIdx.
    /// The dependencies are ordered by InputIdx. Contains None if argument is currently not
    /// provided in the graph, Some(Output) otherwise.
    pub fn get_transform_dependencies(&self, idx: &TransformIdx) -> Vec<Option<Output>> {
        let t = self.get_transform(idx)
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

    /// Add a transform and return its identifier TransformIdx.
    pub fn add_transform(&mut self, t: &'t Transformation<T, E>) -> TransformIdx {
        let idx = self.new_transform_idx();
        self.transforms.insert(idx, t);
        idx
    }

    /// Connect an output to an input.
    /// Returns an error if cycle is created or if output or input does not exist.
    pub fn connect(&mut self, output: Output, input: Input) -> Result<(), DSTError> {
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
        if self.output_exists(&output) {
            let idx = self.new_output_id();
            self.outputs.insert(idx, output);
            Ok(idx)
        } else {
            Err(DSTError::InvalidOutput(format!(
                "{:?} does not exist in this graph!",
                output
            )))
        }
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
            if dep.t_idx == input.t_idx {
                return true;
            }
        }
        false
    }

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

    fn _dependencies(&'t self, output: Output) -> DependencyIter<'t, T, E> {
        DependencyIter {
            dst: self,
            stack: vec![output],
            completed_stack: vec![],
        }
    }

    pub fn dependencies(
        &'t self,
        output_id: &OutputId,
    ) -> Result<DependencyIter<'t, T, E>, DSTError> {
        self.outputs
            .get(output_id)
            .ok_or_else(|| {
                DSTError::MissingOutputID(format!("Output ID {:?} not found!", output_id))
            })
            .map(|output| self._dependencies(*output))
    }

    fn _compute(&self, output: Output) -> Result<T, DSTError> {
        let t = self.get_transform(&output.t_idx).ok_or_else(|| {
            DSTError::ComputeError(format!("Tranform {:?} not found!", output.t_idx))
        })?;
        let deps = self.get_transform_dependencies(&output.t_idx);
        let mut op = t.start();
        for parent_output in deps {
            let parent_output = parent_output.ok_or_else(|| {
                DSTError::ComputeError("Missing dependency! Cannot compute.".to_owned())
            })?;
            op.feed(self._compute(parent_output)?);
        }
        match op.call().nth(output.output_i.into()) {
            None => Err(DSTError::ComputeError(
                "No nth output received. This is a bug!".to_owned(),
            )),
            Some(result) => result.map_err(|_err| {
                // TODO: Improve this error message
                DSTError::ComputeError("Computation failed...".to_owned())
            }),
        }
    }

    pub fn compute(&self, output_id: &OutputId) -> Result<T, DSTError> {
        self.outputs
            .get(output_id)
            .ok_or_else(|| {
                DSTError::MissingOutputID(format!("Output ID {:?} not found!", output_id))
            })
            .and_then(|output| self._compute(*output))
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
    T: Clone + VariantName,
{
    type Item = Dependency;
    /// Push all parents on the stack recursively.
    /// If value has no parents, pop the stack and return it.
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(current_output) = self.stack.pop() {
            let mut parent_outputs = self.dst.get_transform_dependencies(&current_output.t_idx);
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
    }
}

/*** Custom deserializer for DST ***/

use serde::de::{self, Deserialize, Deserializer, MapAccess, Visitor};
use std::fmt;
use std::marker::PhantomData;

/// Desiarializer for DST
impl<'de, T, E: 'de> Deserialize<'de> for DST<'de, T, E>
where
    T: 'static + NamedAlgorithms<E>,
    E: 'static + Clone,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Transforms,
            Edges,
            Outputs,
        };

        struct DSTVisitor<T, E> {
            marker: PhantomData<fn() -> (T, E)>,
        };
        impl<T, E> DSTVisitor<T, E> {
            fn new() -> Self {
                DSTVisitor {
                    marker: PhantomData,
                }
            }
        }

        impl<'de, T, E: 'de> Visitor<'de> for DSTVisitor<T, E>
        where
            T: 'static + NamedAlgorithms<E>,
            E: 'static + Clone,
        {
            type Value = DST<'de, T, E>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct DST")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut transforms = None;
                let mut edges = None;
                let mut outputs = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Transforms => {
                            if transforms.is_some() {
                                return Err(de::Error::duplicate_field("transforms"));
                            }
                            transforms = Some(map.next_value()?);
                        }
                        Field::Edges => {
                            if edges.is_some() {
                                return Err(de::Error::duplicate_field("edges"));
                            }
                            edges = Some(map.next_value()?);
                        }
                        Field::Outputs => {
                            if outputs.is_some() {
                                return Err(de::Error::duplicate_field("outputs"));
                            }
                            outputs = Some(map.next_value()?);
                        }
                    }
                }
                let transforms = transforms.ok_or_else(|| de::Error::missing_field("transforms"))?;
                let edges = edges.ok_or_else(|| de::Error::missing_field("edges"))?;
                let outputs = outputs.ok_or_else(|| de::Error::missing_field("outputs"))?;
                Ok(DST {
                    transforms,
                    edges,
                    outputs,
                })
            }
        }

        const FIELDS: &'static [&'static str] = &["transforms", "edges", "outputs"];
        deserializer.deserialize_struct("DST", FIELDS, DSTVisitor::new())
    }
}

impl<'de, T, E> Deserialize<'de> for &'de Transformation<T, E>
where
    T: 'static + NamedAlgorithms<E>,
    E: 'static + Clone,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TransformationVisitor<T, E> {
            marker: PhantomData<fn() -> (T, E)>,
        };
        impl<T, E> TransformationVisitor<T, E> {
            fn new() -> Self {
                TransformationVisitor {
                    marker: PhantomData,
                }
            }
        }

        impl<'de, T, E> Visitor<'de> for TransformationVisitor<T, E>
        where
            T: 'static + NamedAlgorithms<E>,
            E: 'static + Clone,
        {
            type Value = &'de Transformation<T, E>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Transformation")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut name: Option<String> = None;
                while let Some(key) = map.next_key::<String>()? {
                    println!("{}", key);
                    if key == "name" {
                        if name.is_some() {
                            return Err(de::Error::duplicate_field("name"));
                        }
                        name = Some(map.next_value()?);
                    } else {
                        // Ignore next value. Assume discriminant is saved as u64
                        map.next_value::<Vec<u64>>()?;
                    }
                }
                let name = name.ok_or_else(|| de::Error::missing_field("name"))?;
                let transform = T::get_transform(&name)
                    .ok_or_else(|| de::Error::custom("algorithm name not found"))?;
                Ok(transform)
            }
        }
        const FIELDS: &'static [&'static str] = &["name"];
        deserializer.deserialize_struct("Transformation", FIELDS, TransformationVisitor::new())
    }
}
