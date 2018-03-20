extern crate aflak_backend;
pub use self::aflak_backend::*;

use std::borrow::Cow;

/// Define specific types used in the examples
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum AlgoType {
    Integer,
    Image2d,
}

#[derive(Clone, PartialEq, Debug)]
pub enum AlgoContent {
    Integer(u64),
    Image2d(Vec<Vec<f64>>),
}

fn plus1(input: Vec<Cow<AlgoContent>>) -> Vec<AlgoContent> {
    if let AlgoContent::Integer(i) = *input[0] {
        vec![AlgoContent::Integer(i + 1)]
    } else {
        panic!("Expected integer!")
    }
}

pub fn get_plus1_transform() -> Transformation<'static, AlgoContent> {
    Transformation {
        name: "+1",
        input: vec![AlgoType::Integer],
        output: vec![AlgoType::Integer],
        algorithm: plus1,
    }
}

impl TypeContent for AlgoContent {
    type Type = AlgoType;
    fn get_type(&self) -> Self::Type {
        match self {
            &AlgoContent::Integer(_) => AlgoType::Integer,
            &AlgoContent::Image2d(_) => AlgoType::Image2d,
        }
    }
}

impl NamedAlgorithms for AlgoContent {
    fn get_algorithm(s: &str) -> Option<fn(Vec<Cow<AlgoContent>>) -> Vec<AlgoContent>> {
        match s {
            "+1" => Some(plus1),
            _ => None,
        }
    }
}
