pub use aflak_cake::*;
use variant_name::VariantName;

#[derive(Clone, PartialEq, Debug, VariantName, Serialize, Deserialize)]
pub enum AlgoIO {
    Integer(u64),
    Image2d(Vec<Vec<f64>>),
}

pub fn get_plus1_transform() -> Transformation<AlgoIO, !> {
    cake_transform!(plus1<AlgoIO, !>(i: Integer = 0) -> Integer {
        vec![Ok(AlgoIO::Integer(i + 1))]
    })
}

pub fn get_minus1_transform() -> Transformation<AlgoIO, !> {
    cake_transform!(minus1<AlgoIO, !>(i: Integer = 0) -> Integer {
        vec![Ok(AlgoIO::Integer(i - 1))]
    })
}

pub fn get_get1_transform() -> Transformation<AlgoIO, !> {
    cake_constant!(get1, AlgoIO::Integer(1))
}

pub fn get_get_image_transform() -> Transformation<AlgoIO, !> {
    cake_constant!(image, AlgoIO::Image2d(vec![vec![10.0; 10000]; 10000]))
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
