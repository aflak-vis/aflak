#[macro_use]
extern crate lazy_static;
extern crate variant_name;
#[macro_use]
extern crate variant_name_derive;
#[macro_use]
extern crate aflak_cake as cake;
extern crate fitrs;
extern crate serde;
#[macro_use]
extern crate serde_derive;

mod roi;

use std::sync::Arc;
use variant_name::VariantName;

#[derive(Clone, Debug, VariantName, Serialize, Deserialize)]
pub enum IOValue {
    Integer(i64),
    Float(f32),
    Float2([f32; 2]),
    Float3([f32; 3]),
    Str(String),
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    Fits(Arc<fitrs::Fits>),
    Image1d(Vec<f32>),
    Image2d(Vec<Vec<f32>>),
    Image3d(Vec<Vec<Vec<f32>>>),
    Map2dTo3dCoords(Vec<Vec<[f32; 3]>>),
    Roi(roi::ROI),
}

#[derive(Clone, Debug)]
pub enum IOErr {
    NotFound(String),
    FITSErr(String),
    UnexpectedInput(String),
}

lazy_static! {
    pub static ref TRANSFORMATIONS: Vec<cake::Transformation<IOValue, IOErr>> = {
        vec![
            cake_transform!(open_fits<IOValue, IOErr>(path: Str) -> Fits {
                vec![run_open_fits(path)]
            }),
            cake_transform!(fits_to_3d_image<IOValue, IOErr>(fits: Fits) -> Image3d {
                vec![run_fits_to_3d_image(fits)]
            }),
            cake_transform!(slice_3d_to_2d<IOValue, IOErr>(image: Image3d, map: Map2dTo3dCoords) -> Image2d {
                vec![run_slice_3d_to_2d(image, map)]
            }),
            cake_transform!(make_plane3d<IOValue, IOErr>(p0: Float3, dir1: Float3, dir2: Float3, count1: Integer, count2: Integer) -> Map2dTo3dCoords {
                vec![run_make_plane3d(p0, dir1, dir2, *count1, *count2)]
            }),
            cake_transform!(extract_wave<IOValue, IOErr>(image: Image3d, roi: Roi) -> Image1d {
                vec![run_extract_wave(image, roi)]
            }),
        ]
    };
}

impl cake::NamedAlgorithms<IOErr> for IOValue {
    fn get_transform(s: &str) -> Option<&'static cake::Transformation<IOValue, IOErr>> {
        for t in TRANSFORMATIONS.iter() {
            if t.name == s {
                return Some(t);
            }
        }
        None
    }
}

impl cake::DefaultFor for IOValue {
    fn default_for(variant_name: &str) -> Self {
        match variant_name {
            "Integer" => IOValue::Integer(0),
            "Float" => IOValue::Float(0.0),
            "Float2" => IOValue::Float2([0.0; 2]),
            "Float3" => IOValue::Float3([0.0; 3]),
            "Roi" => IOValue::Roi(roi::ROI::All),
            "Str" => IOValue::Str("".to_owned()),
            _ => panic!("Unknown variant name provided: {}.", variant_name),
        }
    }
}

impl cake::EditableVariants for IOValue {
    fn editable_variants() -> &'static [&'static str] {
        &["Integer", "Float", "Float2", "Float3", "Roi", "Str"]
    }
}

/// Open FITS file
fn run_open_fits(path: &str) -> Result<IOValue, IOErr> {
    fitrs::Fits::open(path)
        .map(|fits| IOValue::Fits(Arc::new(fits)))
        .map_err(|err| IOErr::NotFound(err.to_string()))
}

/// Turn a FITS file into a 3D image
fn run_fits_to_3d_image(fits: &Arc<fitrs::Fits>) -> Result<IOValue, IOErr> {
    let image = {
        let primary_hdu = fits
            .get_by_name("FLUX")
            .or_else(|| fits.get(0))
            .ok_or_else(|| {
                IOErr::UnexpectedInput(
                    "Could not find HDU FLUX nor Primary HDU in FITS file. Is the file valid?"
                        .to_owned(),
                )
            })?;
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
                            values.push(*val);
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

/// Slice a 3D image through an arbitrary 2D plane
fn run_slice_3d_to_2d(
    input_img: &Vec<Vec<Vec<f32>>>,
    map: &Vec<Vec<[f32; 3]>>,
) -> Result<IOValue, IOErr> {
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

/// Make a 2D plane slicing the 3D space
/// This is actually a map mapping 2D to 3D coordinates
fn run_make_plane3d(
    p0: &[f32; 3],
    dir1: &[f32; 3],
    dir2: &[f32; 3],
    count1: i64,
    count2: i64,
) -> Result<IOValue, IOErr> {
    let (&[x0, y0, z0], &[dx1, dy1, dz1], &[dx2, dy2, dz2]) = (p0, dir1, dir2);
    let mut map = Vec::with_capacity(count1 as usize);
    for i in 0..count1 {
        let i = i as f32;
        let mut row = Vec::with_capacity(count2 as usize);
        for j in 0..count2 {
            let j = j as f32;
            row.push([
                x0 + i * dx1 + j * dx2,
                y0 + i * dy1 + j * dy2,
                z0 + i * dz1 + j * dz2,
            ]);
        }
        map.push(row);
    }
    Ok(IOValue::Map2dTo3dCoords(map))
}

fn run_extract_wave(image: &Vec<Vec<Vec<f32>>>, roi: &roi::ROI) -> Result<IOValue, IOErr> {
    let mut wave = Vec::with_capacity(image.len());
    for frame in image.iter() {
        let mut res = 0.0;
        for (_, val) in roi.filter(&frame) {
            res += val;
        }
        wave.push(res);
    }
    Ok(IOValue::Image1d(wave))
}

#[cfg(test)]
mod test {
    use super::{
        run_fits_to_3d_image, run_make_plane3d, run_open_fits, run_slice_3d_to_2d, IOValue,
    };
    #[test]
    fn test_open_fits() {
        let path = "test/test.fits";
        if let IOValue::Fits(fits) = run_open_fits(path).unwrap() {
            if let IOValue::Image3d(image3d) = run_fits_to_3d_image(&fits).unwrap() {
                if let IOValue::Map2dTo3dCoords(map) =
                    run_make_plane3d(&[0.0, 0.0, 0.0], &[1.0, 0.5, 0.0], &[0.0, 0.5, 1.0], 10, 20)
                        .unwrap()
                {
                    let _sliced_image = run_slice_3d_to_2d(&image3d, &map);
                    return;
                }
            }
        }
        panic!("Failed somewhere!");
    }
}
