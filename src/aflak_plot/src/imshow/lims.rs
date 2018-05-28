use std::cmp::Ordering;

use ndarray::Array2;

use super::Error;

pub fn get_vmin(image: &Array2<f32>) -> Result<f32, Error> {
    let min = image
        .iter()
        .min_by(|&f1, &f2| float_compare_nan_max(f1, f2));
    if let Some(min) = min {
        Ok(*min)
    } else {
        Err(Error::Msg("Could not get vmin"))
    }
}

pub fn get_vmax(image: &Array2<f32>) -> Result<f32, Error> {
    let max = image
        .iter()
        .max_by(|&f1, &f2| float_compare_nan_min(f1, f2));
    if let Some(max) = max {
        Ok(*max)
    } else {
        Err(Error::Msg("Could not get vmax"))
    }
}

fn float_compare_nan_min(f1: &f32, f2: &f32) -> Ordering {
    PartialOrd::partial_cmp(f1, f2).unwrap_or_else(|| match (f32::is_nan(*f1), f32::is_nan(*f2)) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => unreachable!(),
    })
}

fn float_compare_nan_max(f1: &f32, f2: &f32) -> Ordering {
    PartialOrd::partial_cmp(f1, f2).unwrap_or_else(|| match (f32::is_nan(*f1), f32::is_nan(*f2)) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        _ => unreachable!(),
    })
}
