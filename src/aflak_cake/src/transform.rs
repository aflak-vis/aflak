use std::borrow::Cow;
use std::fmt;
use std::sync::Arc;
use std::time::Instant;
use std::vec;

use boow::Bow;

use super::ConvertibleVariants;
use compute::ComputeError;
use macros::MacroHandle;
use variant_name::VariantName;

/// Static string that identifies a transformation.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FnTransformId(pub &'static str);
/// Static string that identifies a type of a input/output variable.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeId(pub &'static str);

impl TypeId {
    pub fn name(&self) -> &'static str {
        self.0
    }
}
impl FnTransformId {
    pub fn name(&self) -> &'static str {
        self.0
    }
}

/// Algorithm that defines the transformation
pub enum Algorithm<T, E> {
    /// A rust function with a vector of input variables as argument.
    /// Returns a vector of [`Result`], one result for each output.
    Function {
        f: PlainFunction<T, E>,
        /// Transform id
        id: FnTransformId,
        /// Version of the transform
        version: Version,
        description: &'static str,
        /// Inputs of the transformation, may include a default value
        inputs: Vec<TransformInputSlot<T>>,
        /// Outputs of the transformation
        outputs: Vec<TypeId>,
    },
    /// Use this variant for algorithms with no input. Such algorithm will
    /// always return this constant.
    Constant(T),
    Macro {
        handle: MacroHandle,
    },
}

/// Semantic version
#[derive(Copy, Clone)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
}

#[derive(Clone)]
pub struct TransformInputSlot<T> {
    pub type_id: TypeId,
    pub default: Option<T>,
    pub name: &'static str,
}

impl<T> TransformInputSlot<T> {
    pub fn name_with_type(&self) -> String {
        format!("{}: {}", self.name, self.type_id.name())
    }
}

type PlainFunction<T, E> = fn(Vec<Bow<'_, T>>) -> Vec<Result<T, E>>;

impl<T: fmt::Debug, E> fmt::Debug for Algorithm<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Algorithm::Function { f: ref fun, id, .. } => {
                write!(f, "Function({:?} at {:p})", id.name(), fun)
            }
            Algorithm::Constant(ref vec) => write!(f, "Constant({:?})", vec),
            Algorithm::Macro { ref handle } => write!(f, "Macro({:?})", handle.name()),
        }
    }
}

impl<T, E> Clone for Algorithm<T, E>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        use Algorithm::*;
        match *self {
            Function {
                f,
                id,
                version,
                description,
                ref inputs,
                ref outputs,
            } => Function {
                f,
                id,
                version,
                description,
                inputs: inputs.clone(),
                outputs: outputs.clone(),
            },
            Constant(ref t) => Constant(t.clone()),
            Macro { ref handle } => Macro {
                handle: handle.clone(),
            },
        }
    }
}

/// A transformation defined by an [`Algorithm`], with a determined number of
/// inputs and outputs.
pub struct Transform<T, E> {
    updated_on: Instant,
    /// Algorithm defining the transformation
    algorithm: Algorithm<T, E>,
}

impl<T: fmt::Debug, E> fmt::Debug for Transform<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Transform {{ updated_on: {:?}, algorithm: {:?} }}",
            self.updated_on, self.algorithm
        )
    }
}

impl<T, E> Clone for Transform<T, E>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            updated_on: self.updated_on,
            algorithm: self.algorithm.clone(),
        }
    }
}

impl<T, E> Transform<T, E> {
    pub fn updated_on(&self) -> Instant {
        self.updated_on
    }
    pub fn algorithm(&self) -> &Algorithm<T, E> {
        &self.algorithm
    }
}

/// Result of [`Transform::start`].
///
/// Stores the state of the transformation just before it is called.
pub struct TransformCaller<'a, 'i, T: 'a + 'i, E: 'a> {
    expected_input_types: vec::IntoIter<TypeId>,
    algorithm: &'a Algorithm<T, E>,
    input: Vec<Bow<'i, T>>,
}

impl<T, E> Transform<T, E> {
    pub fn from_algorithm(algorithm: Algorithm<T, E>) -> Self {
        Self {
            updated_on: Instant::now(),
            algorithm,
        }
    }

    /// Create a new Transform always returning a single constant
    pub fn new_constant(t: T) -> Self {
        Self {
            updated_on: Instant::now(),
            algorithm: Algorithm::Constant(t),
        }
    }

    /// Create a transformation from a macro
    pub fn from_macro(handle: MacroHandle) -> Self {
        Self {
            updated_on: Instant::now(),
            algorithm: Algorithm::Macro { handle },
        }
    }

    /// Set this transformation to the given constant value.
    pub fn set_constant(&mut self, t: T) {
        self.updated_on = Instant::now();
        self.algorithm = Algorithm::Constant(t);
    }

    pub fn input_types(&self) -> Vec<TypeId> {
        match self.algorithm {
            Algorithm::Function { ref inputs, .. } => {
                inputs.iter().map(|input| input.type_id).collect()
            }
            Algorithm::Constant(_) => vec![],
            Algorithm::Macro { ref handle } => handle.input_types(),
        }
    }

    pub fn inputs(&self) -> Bow<'_, Vec<TransformInputSlot<T>>> {
        match self.algorithm {
            Algorithm::Function { ref inputs, .. } => Bow::Borrowed(inputs),
            Algorithm::Constant(_) => Bow::Owned(vec![]),
            Algorithm::Macro { ref handle } => Bow::Owned(handle.inputs()),
        }
    }

    /// Ready the transformation to be called.
    pub fn start(&self) -> TransformCaller<T, E> {
        TransformCaller {
            expected_input_types: self.input_types().into_iter(),
            algorithm: &self.algorithm,
            input: Vec::new(),
        }
    }

    /// Check that input exists for the transform
    pub fn input_exists(&self, input_i: usize) -> bool {
        input_i < self.input_types().len()
    }

    /// Return nth input type. Panic if input_i > self.input.len()
    pub fn nth_input_type(&self, input_i: usize) -> TypeId {
        self.input_types()[input_i]
    }
}

impl<T, E> Transform<T, E>
where
    T: Clone,
{
    pub fn defaults(&self) -> Vec<Option<T>> {
        match self.algorithm {
            Algorithm::Function { ref inputs, .. } => inputs
                .iter()
                .map(|input| input.default.as_ref().cloned())
                .collect(),
            Algorithm::Constant(_) => vec![],
            Algorithm::Macro { ref handle } => handle.defaults(),
        }
    }
}

impl<T, E> Transform<T, E>
where
    T: VariantName,
{
    pub fn outputs(&self) -> Vec<TypeId> {
        match self.algorithm {
            Algorithm::Function { ref outputs, .. } => outputs.to_vec(),
            Algorithm::Constant(ref t) => vec![TypeId(t.variant_name())],
            Algorithm::Macro { ref handle } => handle.outputs(),
        }
    }

    /// Check that output exists for the transform
    pub fn output_exists(&self, output_i: usize) -> bool {
        output_i < self.outputs().len()
    }

    /// Return nth output type. Panic if output_i > self.output.len()
    pub fn nth_output_type(&self, output_i: usize) -> TypeId {
        self.outputs()[output_i]
    }

    pub fn name(&self) -> Cow<'static, str> {
        match self.algorithm {
            Algorithm::Function { id, .. } => Cow::Borrowed(id.name()),
            Algorithm::Constant(ref t) => Cow::Borrowed(t.variant_name()),
            Algorithm::Macro { ref handle } => Cow::Owned(handle.name()),
        }
    }

    pub fn description(&self) -> Cow<'static, str> {
        match self.algorithm {
            Algorithm::Function { description, .. } => Cow::Borrowed(description),
            Algorithm::Constant(ref t) => {
                Cow::Owned(format!("Constant variable of type '{}'", t.variant_name()))
            }
            Algorithm::Macro { ref handle } => {
                Cow::Owned(format!("Macro with name '{}'", handle.name()))
            }
        }
    }
}

impl<'a, 'i, T, E> TransformCaller<'a, 'i, T, E>
where
    T: VariantName + ConvertibleVariants,
{
    /// Feed next argument to transformation. Expect a reference as input.
    /// Panic if expected type is not provided or if too many arguments are supplied.
    pub fn feed(&mut self, input: &'i T) {
        let expected_type = self
            .expected_input_types
            .next()
            .expect("Not all type consumed")
            .0;
        let input_type = input.variant_name();
        if let Some(converted_input) = T::convert(input_type, expected_type, input) {
            self.input.push(converted_input);
        } else {
            panic!("Cannot convert '{}' to '{}'", input_type, expected_type)
        }
    }
}

#[derive(Debug)]
pub enum CallError<E> {
    FunctionError(E),
    MacroEvalError(Arc<ComputeError<E>>),
}

impl<'a, 'i, T, E> TransformCaller<'a, 'i, T, E>
where
    T: Clone,
{
    /// Compute the transformation with the provided arguments
    pub fn call(mut self) -> TransformResult<Result<T, CallError<E>>> {
        if self.expected_input_types.next().is_some() {
            panic!("Missing input arguments!");
        } else {
            TransformResult {
                output: match self.algorithm {
                    Algorithm::Function { f, .. } => f(self.input)
                        .into_iter()
                        .map(|r| r.map_err(CallError::FunctionError))
                        .collect::<Vec<_>>()
                        .into_iter(),
                    Algorithm::Constant(c) => vec![Ok(c.clone())].into_iter(),
                    Algorithm::Macro { ref handle } => handle
                        .call(self.input)
                        .into_iter()
                        .map(|e| e.map_err(CallError::MacroEvalError))
                        .collect::<Vec<_>>()
                        .into_iter(),
                },
            }
        }
    }
}

/// Represents the result of a transformation.
pub struct TransformResult<T> {
    output: vec::IntoIter<T>,
}

impl<T> Iterator for TransformResult<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.output.next()
    }
}
