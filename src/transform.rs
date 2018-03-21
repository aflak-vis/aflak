use std::borrow::Cow;
use std::slice;
use std::vec;

pub trait TypeContent: Clone + PartialEq {
    type Type: Clone + PartialEq;
    fn get_type(&self) -> Self::Type;
}

pub trait NamedAlgorithms
where
    Self: Clone,
{
    fn get_algorithm(&str) -> Option<Algorithm<Self>>;
    fn get_transform<T: TypeContent>(&str) -> Option<&'static Transformation<'static, T>>;
}


#[derive(Clone)]
pub enum Algorithm<T: Clone> {
    Function(fn(Vec<Cow<T>>) -> Vec<T>),
    Constant(Vec<T>),
}

#[derive(Clone)]
pub struct Transformation<'de, T: TypeContent> {
    pub name: &'de str,
    pub input: Vec<T::Type>,
    pub output: Vec<T::Type>,
    pub algorithm: Algorithm<T>,
}

pub struct TransformationCaller<'a, 'b, T: 'a + 'b + TypeContent> {
    expected_input_types: slice::Iter<'a, T::Type>,
    algorithm: &'a Algorithm<T>,
    input: Vec<Cow<'b, T>>,
}

impl<'de, T: TypeContent> Transformation<'de, T> {
    pub fn start(&self) -> TransformationCaller<T> {
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
    pub fn nth_output_type(&self, output_i: usize) -> &T::Type {
        &self.output[output_i]
    }

    /// Return nth input type. Panic if input_i > self.input.len()
    pub fn nth_input_type(&self, input_i: usize) -> &T::Type {
        &self.input[input_i]
    }
}

impl<'a, 'b, T: TypeContent> TransformationCaller<'a, 'b, T> {
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
        let expected_type = self.expected_input_types
            .next()
            .expect("Not all type consumed");
        if &input.get_type() != expected_type {
            panic!("Wrong type on feeding algorithm!");
        }
    }

    /// Compute the transformation with the provided arguments
    pub fn call(mut self) -> TransformationResult<T> {
        if self.expected_input_types.next().is_some() {
            panic!("Missing input arguments!");
        } else {
            TransformationResult {
                output: match self.algorithm {
                    &Algorithm::Function(f) => f(self.input).into_iter(),
                    &Algorithm::Constant(ref c) => c.clone().into_iter(),
                },
            }
        }
    }
}

pub struct TransformationResult<T> {
    output: vec::IntoIter<T>,
}

impl<T> Iterator for TransformationResult<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.output.next()
    }
}
