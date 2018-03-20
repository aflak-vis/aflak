extern crate aflak_backend;
use aflak_backend::*;

use std::borrow::Cow;

/// Define specific types used in the examples
#[derive(PartialEq, Debug)]
pub enum AlgoType {
    Integer,
    Image2d,
}

#[derive(Clone, PartialEq, Debug)]
pub enum AlgoContent {
    Integer(u64),
    Image2d(Vec<Vec<f64>>),
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

fn plus1(input: Vec<Cow<AlgoContent>>) -> Vec<AlgoContent> {
    if let AlgoContent::Integer(i) = *input[0] {
        vec![AlgoContent::Integer(i + 1)]
    } else {
        panic!("Expected integer!")
    }
}

#[test]
fn test_plus1() {
    let plus1transform = Transformation {
        input: vec![AlgoType::Integer],
        output: vec![AlgoType::Integer],
        algorithm: plus1,
    };

    let mut caller = plus1transform.start();
    caller.feed(&AlgoContent::Integer(1));
    let mut ret = caller.call();
    assert_eq!(ret.next(), Some(AlgoContent::Integer(2)));
}
