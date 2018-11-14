use std::borrow::Cow;
use std::fmt;
use std::slice;
use std::vec;

use variant_name::VariantName;

use macros::{Macro, MacroEvaluationError};

/// Static string that identifies a transformation.
pub type TransformId = &'static str;
/// Static string that identifies a type of a input/output variable.
pub type TypeId = &'static str;

/// Algorithm that defines the transformation
#[derive(Clone)]
pub enum Algorithm<'t, T: Clone + 't, E: 't> {
    /// A rust function with a vector of input variables as argument.
    /// Returns a vector of [`Result`], one result for each output.
    Function(PlainFunction<T, E>),
    Macro(&'t Macro<'t, T, E>),
    /// Use this variant for algorithms with no input. Such algorithm will
    /// always return this constant.
    Constant(Vec<T>),
}

type PlainFunction<T, E> = fn(Vec<Cow<T>>) -> Vec<Result<T, E>>;

impl<'t, T: Clone + fmt::Debug, E: fmt::Debug> fmt::Debug for Algorithm<'t, T, E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Algorithm::Function(ref fun) => write!(f, "Function({:p})", fun),
            Algorithm::Macro(m) => write!(f, "Macro({:?})", m),
            Algorithm::Constant(ref vec) => write!(f, "Constant({:?})", vec),
        }
    }
}

/// A transformation defined by an [`Algorithm`], with a determined number of
/// inputs and outputs.
#[derive(Clone, Debug)]
pub struct Transformation<'t, T: Clone + 't, E: 't> {
    /// Transformation name
    pub name: TransformId,
    pub description: Cow<'static, str>,
    /// Inputs of the transformation, may include a default value
    pub input: Vec<(TypeId, Option<T>)>,
    /// Outputs of the transformation
    pub output: Vec<TypeId>,
    /// Algorithm defining the transformation
    pub algorithm: Algorithm<'t, T, E>,
}

/// Result of [`Transformation::start`].
///
/// Stores the state of the transformation just before it is called.
pub struct TransformationCaller<'a, 't: 'a, T: 't + Clone, E: 't> {
    expected_input_types: slice::Iter<'a, (TypeId, Option<T>)>,
    algorithm: &'a Algorithm<'t, T, E>,
    input: Vec<Cow<'a, T>>,
}

impl<'t, T, E> Transformation<'t, T, E>
where
    T: Clone + VariantName,
{
    /// Create a new Transformation always returning a single constant
    pub fn new_constant(t: T) -> Self {
        Self {
            name: t.variant_name(),
            description: Cow::Owned(format!("Constant variable of type '{}'", t.variant_name())),
            input: vec![],
            output: vec![t.variant_name()],
            algorithm: Algorithm::Constant(vec![t]),
        }
    }

    pub fn new_macro(macr: &'t Macro<'t, T, E>) -> Self {
        Self {
            name: "Macro",
            description: Cow::Borrowed("Macro"),
            input: macr
                .inputs()
                .iter()
                .map(|(_, type_id, default)| (*type_id, default.clone()))
                .collect(),
            output: macr.outputs(),
            algorithm: Algorithm::Macro(macr),
        }
    }

    /// Set this transformation to the given constant value.
    pub fn set_constant(&mut self, t: T) {
        self.name = t.variant_name();
        self.input = vec![];
        self.output = vec![t.variant_name()];
        self.algorithm = Algorithm::Constant(vec![t])
    }
}

impl<'t, T, E> Transformation<'t, T, E>
where
    T: Clone,
{
    /// Ready the transformation to be called.
    pub fn start(&self) -> TransformationCaller<'t, '_, T, E> {
        TransformationCaller {
            expected_input_types: self.input.iter(),
            algorithm: &self.algorithm,
            input: Vec::new(),
        }
    }

    /// Check that output exists for the transform
    pub fn output_exists(&self, output_i: usize) -> bool {
        output_i < self.output.len()
    }

    /// Check that input exists for the transform
    pub fn input_exists(&self, input_i: usize) -> bool {
        input_i < self.input.len()
    }

    /// Return nth output type. Panic if output_i > self.output.len()
    pub fn nth_output_type(&self, output_i: usize) -> TypeId {
        self.output[output_i]
    }

    /// Return nth input type. Panic if input_i > self.input.len()
    pub fn nth_input_type(&self, input_i: usize) -> TypeId {
        self.input[input_i].0
    }
}

impl<'a, 'b, T, E> TransformationCaller<'a, 'b, T, E>
where
    T: Clone + VariantName,
{
    /// Feed next argument to transformation.
    pub fn feed(&mut self, input: T) {
        self.check_type(&input);
        self.input.push(Cow::Owned(input));
    }

    /// Feed next argument to transformation. Expect a reference as input.
    pub fn feed_ref(&mut self, input: &'b T) {
        self.check_type(input);
        self.input.push(Cow::Borrowed(input));
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

impl<'a, 'b, T, E> TransformationCaller<'a, 'b, T, E>
where
    T: Clone + VariantName + Send + Sync + 'a,
    E: 'a + Send + From<MacroEvaluationError<E>>,
{
    /// Compute the transformation with the provided arguments
    pub fn call(mut self) -> TransformationResult<Result<T, E>> {
        if self.expected_input_types.next().is_some() {
            panic!("Missing input arguments!");
        } else {
            TransformationResult {
                output: match *self.algorithm {
                    Algorithm::Function(f) => f(self.input).into_iter(),
                    Algorithm::Macro(m) => m.call(self.input).into_iter(),
                    Algorithm::Constant(ref c) => c
                        .clone()
                        .into_iter()
                        .map(Ok)
                        .collect::<Vec<_>>()
                        .into_iter(),
                },
            }
        }
    }
}

/// Represents the result of a transformation.
pub struct TransformationResult<T> {
    output: vec::IntoIter<T>,
}

impl<T> Iterator for TransformationResult<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.output.next()
    }
}
