use std::cmp::Ordering;

use ndarray::{self, ArrayBase};

use super::Error;

pub fn get_vmin<S, D>(image: &ArrayBase<S, D>) -> Result<f32, Error>
where
    S: ndarray::Data<Elem = f32>,
    D: ndarray::Dimension,
{
    let min = image
        .iter()
        .min_by(|&&f1, &&f2| float_compare_nan_max(f1, f2));
    if let Some(min) = min {
        Ok(*min)
    } else {
        Err(Error::Msg("Could not get vmin"))
    }
}

pub fn get_vmax<S, D>(image: &ArrayBase<S, D>) -> Result<f32, Error>
where
    S: ndarray::Data<Elem = f32>,
    D: ndarray::Dimension,
{
    let max = image
        .iter()
        .max_by(|&&f1, &&f2| float_compare_nan_min(f1, f2));
    if let Some(max) = max {
        Ok(*max)
    } else {
        Err(Error::Msg("Could not get vmax"))
    }
}

pub fn get_vmed_from_normalized_image<S, D>(image: &ArrayBase<S, D>) -> Result<f32, Error>
where
    S: ndarray::Data<Elem = f32>,
    D: ndarray::Dimension,
{
    let mut data = Vec::new();
    let vmax = get_vmax(image);
    let vmin = get_vmin(image);
    if let (Ok(vmax), Ok(vmin)) = (vmax, vmin) {
        for i in image.iter() {
            if !i.is_nan() {
                data.push((i - vmin) / (vmax - vmin));
            }
        }
        data.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mid = data.len() / 2;
        let med = if data.len() % 2 == 0 {
            (data[mid] + data[mid - 1]) / 2.0
        } else {
            data[mid]
        };
        data.clear();
        Ok(med)
    } else {
        Err(Error::Msg("Could not get vmed from normalized image"))
    }
}

pub fn get_vmad_from_normalized_image<S, D>(image: &ArrayBase<S, D>) -> Result<f32, Error>
where
    S: ndarray::Data<Elem = f32>,
    D: ndarray::Dimension,
{
    let mut data = Vec::new();
    let vmax = get_vmax(image);
    let vmin = get_vmin(image);
    let vmed = get_vmed_from_normalized_image(image);
    if let (Ok(vmax), Ok(vmin), Ok(vmed)) = (vmax, vmin, vmed) {
        for i in image.iter() {
            if !i.is_nan() {
                data.push(((i - vmin) / (vmax - vmin) - vmed).abs());
            }
        }
        data.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mid = data.len() / 2;
        let med = if data.len() % 2 == 0 {
            (data[mid] + data[mid - 1]) / 2.0
        } else {
            data[mid]
        };
        data.clear();
        Ok(med)
    } else {
        Err(Error::Msg("Could not get vmad from normalized image"))
    }
}

fn float_compare_nan_min(f1: f32, f2: f32) -> Ordering {
    PartialOrd::partial_cmp(&f1, &f2).unwrap_or_else(|| match (f32::is_nan(f1), f32::is_nan(f2)) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => unreachable!(),
    })
}

fn float_compare_nan_max(f1: f32, f2: f32) -> Ordering {
    PartialOrd::partial_cmp(&f1, &f2).unwrap_or_else(|| match (f32::is_nan(f1), f32::is_nan(f2)) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        _ => unreachable!(),
    })
}
