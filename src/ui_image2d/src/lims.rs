use std::cmp::Ordering;

use super::Error;

pub fn get_vmin(image: &Vec<Vec<f32>>) -> Result<f32, Error> {
    let min = image
        .iter()
        .map(|row| row.iter().min_by(|&f1, &f2| float_compare_nan_max(f1, f2)))
        .collect::<Vec<_>>()
        .into_iter()
        .min_by(|some_f1, some_f2| match (some_f1, some_f2) {
            (Some(f1), Some(f2)) => float_compare_nan_max(f1, f2),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            _ => Ordering::Equal,
        });
    if let Some(Some(min)) = min {
        Ok(*min)
    } else {
        Err(Error::Msg("Could not get vmin"))
    }
}

pub fn get_vmax(image: &Vec<Vec<f32>>) -> Result<f32, Error> {
    let max = image
        .iter()
        .map(|row| row.iter().max_by(|&f1, &f2| float_compare_nan_min(f1, f2)))
        .collect::<Vec<_>>()
        .into_iter()
        .max_by(|some_f1, some_f2| match (some_f1, some_f2) {
            (Some(f1), Some(f2)) => float_compare_nan_min(f1, f2),
            (Some(_), None) => Ordering::Greater,
            (None, Some(_)) => Ordering::Less,
            _ => Ordering::Equal,
        });
    if let Some(Some(max)) = max {
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
