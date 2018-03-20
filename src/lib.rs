use std::borrow::Cow;

#[derive(PartialEq, Debug)]
pub enum Type {
    Integer,
    Image2d,
}

type Algorithm = fn(Vec<Cow<TypeContent>>) -> Vec<TypeContent>;

pub struct Transformation {
    pub input: Vec<Type>,
    pub output: Vec<Type>,
    pub algorithm: Algorithm,
}

#[derive(Clone, PartialEq, Debug)]
pub enum TypeContent {
    Integer(u64),
    Image2d(Vec<Vec<f64>>),
}

impl TypeContent {
    fn get_type(&self) -> Type {
        match self {
            &TypeContent::Integer(_) => Type::Integer,
            &TypeContent::Image2d(_) => Type::Image2d,
        }
    }
}

use std::slice;

pub struct TransformationCaller<'a, 'b> {
    expected_input_types: slice::Iter<'a, Type>,
    algorithm: &'a Algorithm,
    input: Vec<Cow<'b, TypeContent>>,
}

impl Transformation {
    pub fn start(&self) -> TransformationCaller {
        TransformationCaller {
            expected_input_types: self.input.iter(),
            algorithm: &self.algorithm,
            input: Vec::new(),
        }
    }
}


impl<'a, 'b> TransformationCaller<'a, 'b> {
    pub fn feed(&mut self, input: &'b TypeContent) {
        let expected_type = self.expected_input_types.next().expect("Not all type consumed");
        if &input.get_type() != expected_type {
            panic!("Wrong type on feeding algorithm!");
        } else {
            self.input.push(Cow::Borrowed(input));
        }
    }

    pub fn call(mut self) -> TransformationResult {
        if self.expected_input_types.next().is_some() {
            panic!("Missing input arguments!");
        } else {
            TransformationResult {
                output: (self.algorithm)(self.input).into_iter()
            }
        }
    }
}

use std::vec;

pub struct TransformationResult {
    output: vec::IntoIter<TypeContent>,
}

impl Iterator for TransformationResult {
    type Item = TypeContent;
    fn next(&mut self) -> Option<TypeContent> {
        self.output.next()
    }
}
