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

fn minus1(input: Vec<Cow<AlgoContent>>) -> Vec<AlgoContent> {
    if let AlgoContent::Integer(i) = *input[0] {
        vec![AlgoContent::Integer(i - 1)]
    } else {
        panic!("Expected integer!")
    }
}

fn get1(_: Vec<Cow<AlgoContent>>) -> Vec<AlgoContent> {
    vec![AlgoContent::Integer(1)]
}

fn get_image(_: Vec<Cow<AlgoContent>>) -> Vec<AlgoContent> {
    vec![AlgoContent::Image2d(vec![vec![10.0; 10000]; 10000])]
}

pub fn get_plus1_transform() -> Transformation<'static, AlgoContent> {
    Transformation {
        name: "+1",
        input: vec![AlgoType::Integer],
        output: vec![AlgoType::Integer],
        algorithm: Algorithm::Function(plus1),
    }
}

pub fn get_minus1_transform() -> Transformation<'static, AlgoContent> {
    Transformation {
        name: "-1",
        input: vec![AlgoType::Integer],
        output: vec![AlgoType::Integer],
        algorithm: Algorithm::Function(minus1),
    }
}

pub fn get_get1_transform() -> Transformation<'static, AlgoContent> {
    Transformation {
        name: "1",
        input: vec![],
        output: vec![AlgoType::Integer],
        algorithm: Algorithm::Function(get1),
    }
}

pub fn get_get_image_transform() -> Transformation<'static, AlgoContent> {
    Transformation {
        name: "image",
        input: vec![],
        output: vec![AlgoType::Image2d],
        algorithm: Algorithm::Function(get_image),
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

lazy_static! {
    pub static ref TRANSFORMATIONS: Vec<Transformation<'static, AlgoContent>> = {
        vec![
            get_plus1_transform(),
            get_minus1_transform(),
            get_get1_transform(),
            get_get_image_transform(),
        ]
    };
}

impl NamedAlgorithms for AlgoContent {
    fn get_transform(s: &str) -> Option<&'static Transformation<'static, AlgoContent>> {
        for t in TRANSFORMATIONS.iter() {
            if t.name == s {
                return Some(t);
            }
        }
        None
    }
}
