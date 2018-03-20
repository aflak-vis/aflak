extern crate aflak_backend;
pub use self::aflak_backend::*;

/// Define specific types used in the examples
#[derive(Clone, PartialEq, Debug)]
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
