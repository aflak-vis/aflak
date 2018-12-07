use std::error;
use std::fmt;

use fitrs::FitsDataArray;
use ndarray::{self, ArrayD, IxDyn};

pub trait FitsDataToArray<Dimension> {
    type Target;

    fn to_array(&self) -> Result<Self::Target, FitsArrayReadError>;
}

#[derive(Debug)]
pub enum FitsArrayReadError {
    UnexpectedDimension { expected: usize, got: usize },
    ShapeError(ndarray::ShapeError),
    UnsupportedData(&'static str),
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
            FitsArrayReadError::UnsupportedData(format) => {
                write!(f, "Unsupported data array format: '{}'.", format)
            }
        }
    }
}

impl error::Error for FitsArrayReadError {
    fn description(&self) -> &'static str {
        "FitsArrayReadError"
    }
}

impl FitsDataToArray<IxDyn> for FitsDataArray<f32> {
    type Target = ArrayD<f32>;

    fn to_array(&self) -> Result<ArrayD<f32>, FitsArrayReadError> {
        let sh: Vec<_> = self.shape.iter().rev().cloned().collect();
        ArrayD::from_shape_vec(sh, self.data.clone()).map_err(FitsArrayReadError::ShapeError)
    }
}

impl FitsDataToArray<IxDyn> for FitsDataArray<f64> {
    type Target = ArrayD<f32>;

    fn to_array(&self) -> Result<ArrayD<f32>, FitsArrayReadError> {
        let sh: Vec<_> = self.shape.iter().rev().cloned().collect();
        ArrayD::from_shape_vec(sh, self.data.iter().map(|f| *f as f32).collect())
            .map_err(FitsArrayReadError::ShapeError)
    }
}

impl FitsDataToArray<IxDyn> for FitsDataArray<Option<i32>> {
    type Target = ArrayD<f32>;

    fn to_array(&self) -> Result<ArrayD<f32>, FitsArrayReadError> {
        let sh: Vec<_> = self.shape.iter().rev().cloned().collect();
        ArrayD::from_shape_vec(
            sh,
            self.data
                .iter()
                .map(|some_int| {
                    if let Some(int) = some_int {
                        *int as f32
                    } else {
                        ::std::f32::NAN
                    }
                })
                .collect(),
        )
        .map_err(FitsArrayReadError::ShapeError)
    }
}

impl FitsDataToArray<IxDyn> for FitsDataArray<Option<u32>> {
    type Target = ArrayD<f32>;

    fn to_array(&self) -> Result<ArrayD<f32>, FitsArrayReadError> {
        let sh: Vec<_> = self.shape.iter().rev().cloned().collect();
        ArrayD::from_shape_vec(
            sh,
            self.data
                .iter()
                .map(|some_int| {
                    if let Some(int) = some_int {
                        *int as f32
                    } else {
                        ::std::f32::NAN
                    }
                })
                .collect(),
        )
        .map_err(FitsArrayReadError::ShapeError)
    }
}
