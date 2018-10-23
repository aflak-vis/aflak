#[macro_use]
extern crate lazy_static;
extern crate variant_name;
#[macro_use]
extern crate variant_name_derive;
#[macro_use]
extern crate aflak_cake as cake;
extern crate fitrs;
#[macro_use]
extern crate ndarray;
extern crate serde;
#[macro_use]
extern crate serde_derive;

mod export;
mod roi;
mod unit;

pub use export::ExportError;
pub use unit::{Dimensioned, Unit};

use std::path::{Path, PathBuf};
use std::sync::Arc;

use ndarray::{Array1, Array2, Array3};
use variant_name::VariantName;

#[derive(Clone, Debug, VariantName, Serialize, Deserialize)]
pub enum IOValue {
    Integer(i64),
    Float(f32),
    Float2([f32; 2]),
    Float3([f32; 3]),
    Str(String),
    Path(PathBuf),
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    Fits(Arc<fitrs::Fits>),
    Image1d(Dimensioned<Array1<f32>>),
    Image2d(Dimensioned<Array2<f32>>),
    Image3d(Dimensioned<Array3<f32>>),
    Map2dTo3dCoords(Array2<[f32; 3]>),
    Roi(roi::ROI),
}

impl PartialEq for IOValue {
    fn eq(&self, val: &Self) -> bool {
        use IOValue::*;
        match (self, val) {
            (Integer(i1), Integer(i2)) => i1 == i2,
            (Float(f1), Float(f2)) => f1 == f2,
            (Float2(f1), Float2(f2)) => f1 == f2,
            (Float3(f1), Float3(f2)) => f1 == f2,
            (Str(s1), Str(s2)) => s1 == s2,
            (Image1d(i1), Image1d(i2)) => i1 == i2,
            (Image2d(i1), Image2d(i2)) => i1 == i2,
            (Image3d(i1), Image3d(i2)) => i1 == i2,
            (Map2dTo3dCoords(m1), Map2dTo3dCoords(m2)) => m1 == m2,
            (Roi(r1), Roi(r2)) => r1 == r2,
            (Path(p1), Path(p2)) => p1 == p2,
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub enum IOErr {
    NotFound(String),
    FITSErr(String),
    UnexpectedInput(String),
    ShapeError(ndarray::ShapeError),
}

lazy_static! {
    pub static ref TRANSFORMATIONS: Vec<cake::Transformation<IOValue, IOErr>> = {
        vec![
            cake_transform!(
                "Open FITS file from a Path.",
                open_fits<IOValue, IOErr>(path: Path) -> Fits {
                    vec![run_open_fits(path)]
                }
            ),
            cake_transform!(
                "Extract 3D dataset from FITS file.",
                fits_to_3d_image<IOValue, IOErr>(fits: Fits) -> Image3d {
                    vec![run_fits_to_3d_image(fits)]
                }
            ),
            cake_transform!(
                "Slice an arbitrary plane through a 3D dataset and return the slice.",
                slice_3d_to_2d<IOValue, IOErr>(image: Image3d, map: Map2dTo3dCoords) -> Image2d {
                    vec![run_slice_3d_to_2d(image, map)]
                }
            ),
            cake_transform!(
                "Make a 2D mesh on a specific plane.
Parameters:
1. Starting point: (x, y, z)
2. Directional unit d1: (d1_x, d1_y, d1_z)
3. Directional unit d2: (d2_x, d2_y, d2_z)
4. Integer n
5. Integer m

With the above parameters, a 2D mesh with the points as below:
( x + i * d1_x + j * d2_x,
  y + i * d1_y + j * d2_y,
  z + i * d1_z + j * d2_z )
for 0 <= i < n and 0 <= j < m",
                make_plane3d<IOValue, IOErr>(p0: Float3 = [0.0; 3], dir1: Float3 = [0.0, 1.0, 0.0], dir2: Float3 = [0.0, 0.0, 1.0], count1: Integer = 1, count2: Integer = 1) -> Map2dTo3dCoords {
                    vec![run_make_plane3d(p0, dir1, dir2, *count1, *count2)]
                }
            ),
            cake_transform!(
                "Extract waveform from 3D image with the provided region of interest.",
                extract_wave<IOValue, IOErr>(image: Image3d, roi: Roi = roi::ROI::All) -> Image1d {
                    vec![run_extract_wave(image, roi)]
                }
            ),
            cake_transform!(
                "Compose 2 1D-vectors. Parameters: u, v, a, b.
Compute a*u + b*v.",
                linear_composition_1d<IOValue, IOErr>(i1: Image1d, i2: Image1d, coef1: Float = 1.0, coef2: Float = 1.0) -> Image1d {
                    vec![run_linear_composition_1d(i1, i2, *coef1, *coef2)]
                }
            ),
            cake_transform!(
                "Compose 2 2D-vectors. Parameters: u, v, a, b.
Compute a*u + b*v.",
                linear_composition_2d<IOValue, IOErr>(i1: Image2d, i2: Image2d, coef1: Float = 1.0, coef2: Float = 1.0) -> Image2d {
                    vec![run_linear_composition_2d(i1, i2, *coef1, *coef2)]
                }
            ),
            cake_transform!(
                "Make a Float3 from 3 float values.",
                make_float3<IOValue, IOErr>(f1: Float = 0.0, f2: Float = 0.0, f3: Float = 0.0) -> Float3 {
                    vec![run_make_float3(*f1, *f2, *f3)]
                }
            ),
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
            "Path" => IOValue::Path(PathBuf::from("/")),
            _ => panic!("Unknown variant name provided: {}.", variant_name),
        }
    }
}

impl cake::EditableVariants for IOValue {
    fn editable_variants() -> &'static [&'static str] {
        &["Integer", "Float", "Float2", "Float3", "Roi", "Str", "Path"]
    }
}

/// Open FITS file
fn run_open_fits<P: AsRef<Path>>(path: P) -> Result<IOValue, IOErr> {
    fitrs::Fits::open(path)
        .map(|fits| IOValue::Fits(Arc::new(fits)))
        .map_err(|err| IOErr::NotFound(err.to_string()))
}

/// Turn a FITS file into a 3D image
fn run_fits_to_3d_image(fits: &Arc<fitrs::Fits>) -> Result<IOValue, IOErr> {
    let (image, unit) = {
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
        let image = match data {
            &fitrs::FitsData::FloatingPoint32(ref image) => Array3::from_shape_vec(
                (image.shape[2], image.shape[1], image.shape[0]),
                image.data.clone(),
            ).map_err(IOErr::ShapeError)?,
            _ => unimplemented!(),
        };
        let unit =
            if let Some(fitrs::HeaderValue::CharacterString(unit)) = primary_hdu.value("BUNIT") {
                Unit::Custom(unit.to_owned())
            } else {
                Unit::None
            };
        (image, unit)
    };
    Ok(IOValue::Image3d(unit.new(image)))
}

/// Slice a 3D image through an arbitrary 2D plane
fn run_slice_3d_to_2d(
    input_img: &Dimensioned<Array3<f32>>,
    map: &Array2<[f32; 3]>,
) -> Result<IOValue, IOErr> {
    let mut out = Vec::with_capacity(map.len());
    for &[x, y, z] in map {
        // Interpolate to nearest
        let input_img = input_img.scalar();
        let out_val = *input_img
            .get([x as usize, y as usize, z as usize])
            .ok_or_else(|| {
                IOErr::UnexpectedInput(format!(
                    "Input maps to out of bound pixel!: [{}, {}, {}]",
                    x, y, z
                ))
            })?;
        out.push(out_val);
    }
    Array2::from_shape_vec(map.dim(), out)
        .map(|array| IOValue::Image2d(input_img.with_new_value(array)))
        .map_err(IOErr::ShapeError)
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
    let count1 = count1 as usize;
    let count2 = count2 as usize;
    let mut map = Vec::with_capacity(count1 * count2);
    for j in 0..count2 {
        let j = j as f32;
        for i in 0..count1 {
            let i = i as f32;
            map.push([
                x0 + i * dx1 + j * dx2,
                y0 + i * dy1 + j * dy2,
                z0 + i * dz1 + j * dz2,
            ]);
        }
    }
    Array2::from_shape_vec((count1, count2), map)
        .map(IOValue::Map2dTo3dCoords)
        .map_err(IOErr::ShapeError)
}

fn run_extract_wave(image: &Dimensioned<Array3<f32>>, roi: &roi::ROI) -> Result<IOValue, IOErr> {
    let image_val = image.scalar();
    let mut wave = Vec::with_capacity(image_val.len());
    for i in 0..image_val.dim().0 {
        let mut res = 0.0;
        for (_, val) in roi.filter(image_val.slice(s![i, .., ..])) {
            res += val;
        }
        wave.push(res);
    }
    Ok(IOValue::Image1d(
        image.with_new_value(Array1::from_vec(wave)),
    ))
}

fn run_linear_composition_1d(
    i1: &Dimensioned<Array1<f32>>,
    i2: &Dimensioned<Array1<f32>>,
    coef1: f32,
    coef2: f32,
) -> Result<IOValue, IOErr> {
    let out = i1 * coef1 + i2 * coef2;
    Ok(IOValue::Image1d(out))
}

fn run_linear_composition_2d(
    i1: &Dimensioned<Array2<f32>>,
    i2: &Dimensioned<Array2<f32>>,
    coef1: f32,
    coef2: f32,
) -> Result<IOValue, IOErr> {
    let out = i1 * coef1 + i2 * coef2;
    Ok(IOValue::Image2d(out))
}

fn run_make_float3(f1: f32, f2: f32, f3: f32) -> Result<IOValue, IOErr> {
    Ok(IOValue::Float3([f1, f2, f3]))
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
