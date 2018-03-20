use std::borrow::Cow;
use std::slice;
use std::vec;

pub trait TypeContent: Clone + PartialEq {
    type Type: PartialEq;
    fn get_type(&self) -> Self::Type;
}

type Algorithm<T> = fn(Vec<Cow<T>>) -> Vec<T>;

pub struct Transformation<T: TypeContent> {
    pub input: Vec<T::Type>,
    pub output: Vec<T::Type>,
    pub algorithm: Algorithm<T>,
}

pub struct TransformationCaller<'a, 'b, T: 'a + 'b + TypeContent> {
    expected_input_types: slice::Iter<'a, T::Type>,
    algorithm: &'a Algorithm<T>,
    input: Vec<Cow<'b, T>>,
}

impl<T: TypeContent> Transformation<T> {
    pub fn start(&self) -> TransformationCaller<T> {
        TransformationCaller {
            expected_input_types: self.input.iter(),
            algorithm: &self.algorithm,
            input: Vec::new(),
        }
    }
}

impl<'a, 'b, T: TypeContent> TransformationCaller<'a, 'b, T> {
    pub fn feed(&mut self, input: &'b T) {
        let expected_type = self.expected_input_types
            .next()
            .expect("Not all type consumed");
        if &input.get_type() != expected_type {
            panic!("Wrong type on feeding algorithm!");
        } else {
            self.input.push(Cow::Borrowed(input));
        }
    }

    pub fn call(mut self) -> TransformationResult<T> {
        if self.expected_input_types.next().is_some() {
            panic!("Missing input arguments!");
        } else {
            TransformationResult {
                output: (self.algorithm)(self.input).into_iter(),
            }
        }
    }
}

pub struct TransformationResult<T: TypeContent> {
    output: vec::IntoIter<T>,
}

impl<T: TypeContent> Iterator for TransformationResult<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.output.next()
    }
}
