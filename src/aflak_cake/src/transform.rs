use std::borrow::Cow;
use std::fmt;
use std::time::Instant;
use std::vec;

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
        description: &'static str,
        /// Inputs of the transformation, may include a default value
        inputs: Vec<(TypeId, Option<T>)>,
        /// Outputs of the transformation
        outputs: Vec<TypeId>,
    },
    /// Use this variant for algorithms with no input. Such algorithm will
    /// always return this constant.
    Constant(T),
}

type PlainFunction<T, E> = fn(Vec<&T>) -> Vec<Result<T, E>>;

impl<T: fmt::Debug, E> fmt::Debug for Algorithm<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Algorithm::Function { f: ref fun, id, .. } => {
                write!(f, "Function({:?} at {:p})", id.name(), fun)
            }
            Algorithm::Constant(ref vec) => write!(f, "Constant({:?})", vec),
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
    input: Vec<&'i T>,
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

    /// Set this transformation to the given constant value.
    pub fn set_constant(&mut self, t: T) {
        self.updated_on = Instant::now();
        self.algorithm = Algorithm::Constant(t);
    }

    pub fn inputs(&self) -> Vec<TypeId> {
        match self.algorithm {
            Algorithm::Function { ref inputs, .. } => {
                inputs.iter().map(|(type_id, _)| *type_id).collect()
            }
            Algorithm::Constant(_) => vec![],
        }
    }

    /// Ready the transformation to be called.
    pub fn start(&self) -> TransformCaller<T, E> {
        TransformCaller {
            expected_input_types: self.inputs().into_iter(),
            algorithm: &self.algorithm,
            input: Vec::new(),
        }
    }

    /// Check that input exists for the transform
    pub fn input_exists(&self, input_i: usize) -> bool {
        input_i < self.inputs().len()
    }

    /// Return nth input type. Panic if input_i > self.input.len()
    pub fn nth_input_type(&self, input_i: usize) -> TypeId {
        self.inputs()[input_i]
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
                .map(|(_, default)| default.as_ref().cloned())
                .collect(),
            Algorithm::Constant(_) => vec![],
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

    pub fn name(&self) -> &'static str {
        match self.algorithm {
            Algorithm::Function { id, .. } => id.name(),
            Algorithm::Constant(ref t) => t.variant_name(),
        }
    }

    pub fn description(&self) -> Cow<'static, str> {
        match self.algorithm {
            Algorithm::Function { description, .. } => Cow::Borrowed(description),
            Algorithm::Constant(ref t) => {
                Cow::Owned(format!("Constant variable of type '{}'", t.variant_name()))
            }
        }
    }
}

impl<'a, 'i, T, E> TransformCaller<'a, 'i, T, E>
where
    T: VariantName,
{
    /// Feed next argument to transformation. Expect a reference as input.
    pub fn feed(&mut self, input: &'i T) {
        self.check_type(input);
        self.input.push(input);
    }

    /// Panic if expected type is not provided or if too many arguments are supplied.
    fn check_type(&mut self, input: &T) {
        let expected_type = self
            .expected_input_types
            .next()
            .expect("Not all type consumed")
            .0;
        if input.variant_name() != expected_type {
            panic!("Wrong type on feeding algorithm!");
        }
    }
}

impl<'a, 'i, T, E> TransformCaller<'a, 'i, T, E>
where
    T: Clone,
{
    /// Compute the transformation with the provided arguments
    pub fn call(mut self) -> TransformResult<Result<T, E>> {
        if self.expected_input_types.next().is_some() {
            panic!("Missing input arguments!");
        } else {
            TransformResult {
                output: match self.algorithm {
                    Algorithm::Function { f, .. } => f(self.input).into_iter(),
                    Algorithm::Constant(c) => vec![Ok(c.clone())].into_iter(),
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
