pub use aflak_cake::*;
use std::fmt;
use variant_name::VariantName;

#[derive(Clone, PartialEq, Debug, VariantName, Serialize, Deserialize)]
pub enum AlgoIO {
    Integer(u64),
    Image2d(Vec<Vec<f64>>),
}

/// `never` type representing an impossible error (similar to ! in rust nightly)
#[derive(Debug)]
pub enum E {
    MacroError(Box<MacroEvaluationError<E>>),
}

impl PartialEq for E {
    fn eq(&self, _other: &E) -> bool {
        false
    }
}

impl From<MacroEvaluationError<E>> for E {
    fn from(e: MacroEvaluationError<E>) -> E {
        E::MacroError(Box::new(e))
    }
}

impl fmt::Display for E {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            E::MacroError(ref e) => e.fmt(f),
        }
    }
}

pub fn get_plus1_transform() -> Transformation<'static, AlgoIO, E> {
    cake_transform!("Add 1", plus1<AlgoIO, E>(i: Integer = 0) -> Integer {
        vec![Ok(AlgoIO::Integer(i + 1))]
    })
}

pub fn get_minus1_transform() -> Transformation<'static, AlgoIO, E> {
    cake_transform!("Substract 1", minus1<AlgoIO, E>(i: Integer) -> Integer {
        vec![Ok(AlgoIO::Integer(i - 1))]
    })
}

pub fn get_get1_transform() -> Transformation<'static, AlgoIO, E> {
    cake_constant!(get1, AlgoIO::Integer(1))
}

pub fn get_get_image_transform() -> Transformation<'static, AlgoIO, E> {
    cake_constant!(image, AlgoIO::Image2d(vec![vec![10.0; 10000]; 10000]))
}

lazy_static! {
    pub static ref TRANSFORMATIONS: Vec<Transformation<'static, AlgoIO, E>> = {
        vec![
            get_plus1_transform(),
            get_minus1_transform(),
            get_get1_transform(),
            get_get_image_transform(),
        ]
    };
}

impl NamedAlgorithms<E> for AlgoIO {
    fn get_transform(s: &str) -> Option<&'static Transformation<'static, AlgoIO, E>> {
        for t in TRANSFORMATIONS.iter() {
            if t.name == s {
                return Some(t);
            }
        }
        None
    }
}
