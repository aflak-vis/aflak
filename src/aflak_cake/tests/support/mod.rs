pub use aflak_cake::*;
use std::fmt;
use variant_name::VariantName;

#[derive(Clone, PartialEq, Debug, VariantName, Serialize, Deserialize)]
pub enum AlgoIO {
    Integer(u64),
    Float(f64),
    Image2d(Vec<Vec<f64>>),
}

/// `never` type representing an impossible error (similar to ! in rust nightly)
#[derive(Clone, PartialEq, Debug)]
pub enum E {}

impl fmt::Display for E {
    fn fmt(&self, _: &mut fmt::Formatter) -> fmt::Result {
        match *self {}
    }
}

pub fn get_plus1_transform() -> Transform<'static, AlgoIO, E> {
    cake_transform!("Add 1", 1, 0, 0, plus1<AlgoIO, E>(i: Integer = 0) -> Integer {
        vec![Ok(AlgoIO::Integer(i + 1))]
    })
}

pub fn get_minus1_transform() -> Transform<'static, AlgoIO, E> {
    cake_transform!("Substract 1", 1, 0, 0, minus1<AlgoIO, E>(i: Integer) -> Integer {
        vec![Ok(AlgoIO::Integer(i - 1))]
    })
}

pub fn get_divide_by_10_transform() -> Transform<'static, AlgoIO, E> {
    cake_transform!("Divide by 10", 1, 0, 0, divide_by_10<AlgoIO, E>(f: Float) -> Float {
        vec![Ok(AlgoIO::Float(f / 10.0))]
    })
}

pub fn get_get1_transform() -> Transform<'static, AlgoIO, E> {
    Transform::new_constant(AlgoIO::Integer(1))
}

pub fn get_get_image_transform() -> Transform<'static, AlgoIO, E> {
    Transform::new_constant(AlgoIO::Image2d(vec![vec![10.0; 10000]; 10000]))
}

lazy_static! {
    static ref TRANSFORMATIONS: Vec<Transform<'static, AlgoIO, E>> = {
        vec![
            get_plus1_transform(),
            get_minus1_transform(),
            get_get1_transform(),
            get_get_image_transform(),
            get_divide_by_10_transform(),
        ]
    };
    pub static ref TRANSFORMATIONS_REF: &'static [&'static Transform<'static, AlgoIO, E>] = {
        let vec = Box::new(TRANSFORMATIONS.iter().collect::<Vec<_>>());
        let vec = Box::leak(vec);
        vec.as_slice()
    };
}

impl NamedAlgorithms<E> for AlgoIO {
    fn get_transform(s: &str) -> Option<&'static Transform<'static, AlgoIO, E>> {
        for t in TRANSFORMATIONS_REF.iter() {
            if t.name() == s {
                return Some(t);
            }
        }
        None
    }
}

impl ConvertibleVariants for AlgoIO {
    const CONVERTION_TABLE: &'static [ConvertibleVariant<Self>] = &[ConvertibleVariant {
        from: "Integer",
        into: "Float",
        f: integer_to_float,
    }];
}

fn integer_to_float(from: &AlgoIO) -> AlgoIO {
    if let AlgoIO::Integer(int) = from {
        AlgoIO::Float(*int as _)
    } else {
        panic!("Unexpected input!")
    }
}
