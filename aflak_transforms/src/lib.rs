extern crate variant_name;
#[macro_use]
extern crate variant_name_derive;
#[macro_use]
extern crate aflak_cake as cake;
extern crate fitrs;
extern crate serde;

use variant_name::VariantName;
use std::borrow::Cow;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, VariantName)]
pub enum IOValue {
    Integer(i64),
    Float(f64),
    Str(String),
    Fits(Arc<Mutex<fitrs::Fits>>),
    Image1d(Vec<f64>),
    Image2d(Vec<Vec<f64>>),
    Image3d(Vec<Vec<Vec<f64>>>),
    Map2dTo3dCoords(Vec<Vec<[f64; 3]>>),
}

#[derive(Clone, Debug)]
pub enum IOErr {
    NotFound(String),
    FITSErr(String),
    UnexpectedInput(String),
}

/// Open FITS file
fn open_fits(input: Vec<Cow<IOValue>>) -> Vec<Result<IOValue, IOErr>> {
    if let IOValue::Str(ref path) = *input[0] {
        vec![
            fitrs::Fits::open(path)
                .map(|fits| IOValue::Fits(Arc::new(Mutex::new(fits))))
                .map_err(|err| IOErr::NotFound(err.to_string())),
        ]
    } else {
        panic!("Expected path as input!")
    }
}

fn fits_to_3d_image(input: Vec<Cow<IOValue>>) -> Vec<Result<IOValue, IOErr>> {
    fn convert_fits(fits: &Arc<Mutex<fitrs::Fits>>) -> Result<IOValue, IOErr> {
        let image = {
            let mut file = fits.lock().unwrap();
            let primary_hdu = &mut file[0];
            let data = primary_hdu.read_data();
            match data {
                &fitrs::FitsData::FloatingPoint32(ref image) => {
                    let (x_max, y_max, z_max) = (image.shape[0], image.shape[1], image.shape[2]);
                    let mut frames = Vec::with_capacity(z_max);
                    let mut iter = image.data.iter();
                    for _ in 0..z_max {
                        let mut rows = Vec::with_capacity(y_max);
                        for _ in 0..y_max {
                            let mut values = Vec::with_capacity(x_max);
                            for _ in 0..x_max {
                                let val = iter.next().ok_or_else(|| {
                                    IOErr::FITSErr("Unexpected length of in FITS file".to_owned())
                                })?;
                                values.push(*val as f64);
                            }
                            rows.push(values);
                        }
                        frames.push(rows);
                    }
                    frames
                }
                _ => unimplemented!(),
            }
        };
        Ok(IOValue::Image3d(image))
    }

    if let IOValue::Fits(ref fits) = *input[0] {
        vec![convert_fits(fits)]
    } else {
        panic!("Expectect FITS as input")
    }
}

fn slice_3d_to_2d(input: Vec<Cow<IOValue>>) -> Vec<Result<IOValue, IOErr>> {
    fn slice(input_img: &Vec<Vec<Vec<f64>>>, map: &Vec<Vec<[f64; 3]>>) -> Result<IOValue, IOErr> {
        let mut out = Vec::with_capacity(map.len());
        for row in map {
            let mut out_rows = Vec::with_capacity(row.len());
            for &[x, y, z] in row {
                // Interpolate to nearest
                let out_val = *input_img
                    .get(x as usize)
                    .and_then(|f| f.get(y as usize))
                    .and_then(|r| r.get(z as usize))
                    .ok_or_else(|| {
                        IOErr::UnexpectedInput(format!(
                            "Input maps to out of bound pixel!: [{}, {}, {}]",
                            x, y, z
                        ))
                    })?;
                out_rows.push(out_val);
            }
            out.push(out_rows);
        }
        Ok(IOValue::Image2d(out))
    }

    if let (&IOValue::Image3d(ref input_img), &IOValue::Map2dTo3dCoords(ref map)) =
        (&*input[0], &*input[1])
    {
        vec![slice(input_img, map)]
    } else {
        panic!("Unexpected input")
    }
}

fn plane3d(input: Vec<Cow<IOValue>>) -> Vec<Result<IOValue, IOErr>> {
    use IOValue::*;
    if let (
        &Image1d(ref p0),
        &Image1d(ref dir1),
        &Image1d(ref dir2),
        &Integer(count1),
        &Integer(count2),
    ) = (&*input[0], &*input[1], &*input[2], &*input[3], &*input[4])
    {
        match (p0.as_slice(), dir1.as_slice(), dir2.as_slice()) {
            (&[x0, y0, z0], &[dx1, dy1, dz1], &[dx2, dy2, dz2]) => {
                let mut map = Vec::with_capacity(count1 as usize);
                for i in 0..count1 {
                    let i = i as f64;
                    let mut row = Vec::with_capacity(count2 as usize);
                    for j in 0..count2 {
                        let j = j as f64;
                        row.push([
                            x0 + i * dx1 + j * dx2,
                            y0 + i * dy1 + j * dy2,
                            z0 + i * dz1 + j * dz2,
                        ]);
                    }
                    map.push(row);
                }
                vec![Ok(IOValue::Map2dTo3dCoords(map))]
            }
            _ => vec![
                Err(IOErr::UnexpectedInput(
                    "Expected input vectors to have a length of 3 [x, y, z].".to_owned(),
                )),
            ],
        }
    } else {
        panic!("Unexpected input")
    }
}

#[cfg(test)]
mod test {
    use std::borrow::Cow;
    use super::{open_fits, IOValue, fits_to_3d_image, plane3d, slice_3d_to_2d};
    #[test]
    fn test_open_fits() {
        let path = IOValue::Str("/home/malik/workspace/lab/aflak/data/test.fits".to_owned());
        let ret_fits = open_fits(vec![Cow::Owned(path)]);
        let ret_3d_image = fits_to_3d_image(vec![Cow::Borrowed(ret_fits[0].as_ref().unwrap())]);
        let plane = plane3d(vec![
            Cow::Owned(IOValue::Image1d(vec![0.0, 0.0, 0.0])),
            Cow::Owned(IOValue::Image1d(vec![1.0, 0.5, 0.0])),
            Cow::Owned(IOValue::Image1d(vec![0.0, 0.5, 1.0])),
            Cow::Owned(IOValue::Integer(10)),
            Cow::Owned(IOValue::Integer(20)),
        ]);
        let sliced_image = slice_3d_to_2d(vec![
            Cow::Borrowed(ret_3d_image[0].as_ref().unwrap()),
            Cow::Borrowed(plane[0].as_ref().unwrap()),
        ]);
    }
}
