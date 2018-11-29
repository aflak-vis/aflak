use std::error;
use std::fmt;

use fitrs::FitsDataArray;
use ndarray::{self, Array3, Ix3};

pub trait FitsDataToArray<Dimension> {
    type Target;

    fn to_array(&self) -> Result<Self::Target, FitsArrayReadError>;
}

#[derive(Debug)]
pub enum FitsArrayReadError {
    UnexpectedDimension { expected: usize, got: usize },
    ShapeError(ndarray::ShapeError),
    UnsupportedData,
}

impl fmt::Display for FitsArrayReadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FitsArrayReadError::UnexpectedDimension { expected, got } => write!(
                f,
                "Expects a {}-dimensional FITS file. But the input file has {} dimensions.",
                expected, got
            ),
            FitsArrayReadError::ShapeError(ref e) => e.fmt(f),
            FitsArrayReadError::UnsupportedData => write!(f, "Unsupported data array format."),
        }
    }
}

impl error::Error for FitsArrayReadError {
    fn description(&self) -> &'static str {
        "FitsArrayReadError"
    }
}

impl FitsDataToArray<Ix3> for FitsDataArray<f32> {
    type Target = Array3<f32>;

    fn to_array(&self) -> Result<Array3<f32>, FitsArrayReadError> {
        let sh = &self.shape;
        if sh.len() != 3 {
            Err(FitsArrayReadError::UnexpectedDimension {
                expected: 3,
                got: sh.len(),
            })
        } else {
            Array3::from_shape_vec((sh[2], sh[1], sh[0]), self.data.clone())
                .map_err(FitsArrayReadError::ShapeError)
        }
    }
}
