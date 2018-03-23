pub use aflak_cake::*;
use variant_name::VariantName;

#[derive(Clone, PartialEq, Debug, VariantName, Serialize)]
pub enum AlgoIO {
    Integer(u64),
    Image2d(Vec<Vec<f64>>),
}

cake_fn!(plus1<AlgoIO, !>(i: Integer) {
    vec![Ok(AlgoIO::Integer(i + 1))]
});

cake_fn!(minus1<AlgoIO, !>(i: Integer) {
    vec![Ok(AlgoIO::Integer(i - 1))]
});

cake_fn!(get1<AlgoIO, !>() {
    vec![Ok(AlgoIO::Integer(1))]
});

cake_fn!(get_image<AlgoIO, !>() {
    vec![Ok(AlgoIO::Image2d(vec![vec![10.0; 10000]; 10000]))]
});

pub fn get_plus1_transform() -> Transformation<AlgoIO, !> {
    Transformation {
        name: "+1",
        input: vec!["Integer"],
        output: vec!["Integer"],
        algorithm: Algorithm::Function(plus1),
    }
}

pub fn get_minus1_transform() -> Transformation<AlgoIO, !> {
    Transformation {
        name: "-1",
        input: vec!["Integer"],
        output: vec!["Integer"],
        algorithm: Algorithm::Function(minus1),
    }
}

pub fn get_get1_transform() -> Transformation<AlgoIO, !> {
    Transformation {
        name: "1",
        input: vec![],
        output: vec!["Integer"],
        algorithm: Algorithm::Function(get1),
    }
}

pub fn get_get_image_transform() -> Transformation<AlgoIO, !> {
    Transformation {
        name: "image",
        input: vec![],
        output: vec!["Image2d"],
        algorithm: Algorithm::Function(get_image),
    }
}

lazy_static! {
    pub static ref TRANSFORMATIONS: Vec<Transformation<AlgoIO, !>> = {
        vec![
            get_plus1_transform(),
            get_minus1_transform(),
            get_get1_transform(),
            get_get_image_transform(),
        ]
    };
}

impl NamedAlgorithms<!> for AlgoIO {
    fn get_transform(s: &str) -> Option<&'static Transformation<AlgoIO, !>> {
        for t in TRANSFORMATIONS.iter() {
            if t.name == s {
                return Some(t);
            }
        }
        None
    }
}
