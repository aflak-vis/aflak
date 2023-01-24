//! This is the crate containing the definitions of the transformations and
//! data types used in aflak for the astrophysical domain.
//!
//! The crate implements all the required traits for `aflak_cake` to kicks in
//! astrophysical computation.
//!
//! You will want first to check [IOValue](enum.IOValue.html). This is the
//! enumeration that defines all the types used as inputs and outputs of
//! astrophysical transformations.
//!
//! If the output of a transformation should return an error, the
//! [IOErr](enum.IOErr.html) should be used.
//!
//! To add a new transformation, add a new `Transform<IOValue, IOErr>` item to
//! the [TRANSFORMATIONS](struct.TRANSFORMATIONS.html) struct defined using
//! the lazy_static crate.
extern crate lazy_static;
extern crate variant_name;
#[macro_use]
extern crate variant_name_derive;
#[macro_use]
extern crate aflak_cake as cake;
pub extern crate fitrs;
extern crate imgui_tone_curve;
extern crate lab;
extern crate libm;

#[macro_use]
pub extern crate ndarray;
extern crate nalgebra;
extern crate ndarray_parallel;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate rawloader;
extern crate ttk_sys;

mod fits;
#[macro_use]
mod precond;
mod dijkstra;
mod roi;
mod unit;

pub use crate::roi::ROI;
pub use crate::unit::{
    CriticalPoints, DerivedUnit, Dimensioned, Separatrices1Cell, Separatrices1Point, Topology,
    Unit, WcsArray,
};

use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::slice;
use std::sync::Arc;

use imgui_tone_curve::ToneCurveState;
use nalgebra::{DMatrix, DVector, Matrix3, Vector3};
use ndarray::{Array, Array1, Array2, ArrayD, ArrayViewD, Axis, Dimension, ShapeBuilder, Slice};
use ndarray_parallel::prelude::*;
use ttk_sys::Ttk_rs;
use variant_name::VariantName;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PATHS {
    FileList(Vec<PathBuf>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PersistencePairs {
    Pairs(Vec<(i32, i32, f32, f32)>),
}

/// Value used for I/O in astronomical transforms.
///
/// If new use cases arise, please add a new variant to this enumeration.
#[derive(Clone, Debug, VariantName, Serialize, Deserialize)]
pub enum IOValue {
    Integer(i64),
    Float(f32),
    Float2([f32; 2]),
    Float3([f32; 3]),
    Float3x3([[f32; 3]; 3]),
    Str(String),
    Bool(bool),
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    Fits(Arc<fitrs::Fits>),
    Image(WcsArray),
    Map2dTo3dCoords(Array2<[f32; 3]>),
    Roi(roi::ROI),
    ColorLut((usize, Vec<(f32, [u8; 3])>)),
    Paths(PATHS),
    ToneCurve(ToneCurveState),
    PersistencePairs(PersistencePairs),
}

impl PartialEq for IOValue {
    fn eq(&self, val: &Self) -> bool {
        use crate::IOValue::*;
        match (self, val) {
            (Integer(i1), Integer(i2)) => i1 == i2,
            (Float(f1), Float(f2)) => f1 == f2,
            (Float2(f1), Float2(f2)) => f1 == f2,
            (ToneCurve(t1), ToneCurve(t2)) => t1 == t2,
            (PersistencePairs(p1), PersistencePairs(p2)) => p1 == p2,
            (Float3(f1), Float3(f2)) => f1 == f2,
            (Float3x3(f1), Float3x3(f2)) => f1 == f2,
            (Str(s1), Str(s2)) => s1 == s2,
            (Bool(b1), Bool(b2)) => b1 == b2,
            (Image(i1), Image(i2)) => i1 == i2,
            (Map2dTo3dCoords(m1), Map2dTo3dCoords(m2)) => m1 == m2,
            (Roi(r1), Roi(r2)) => r1 == r2,
            (ColorLut(c1), ColorLut(c2)) => *c1 == *c2,
            (Paths(p1), Paths(p2)) => p1 == p2,
            _ => false,
        }
    }
}

/// Error value used for I/O in astronomical transforms.
///
/// If new use cases arise, please add a new variant to this enumeration.
#[derive(Debug)]
pub enum IOErr {
    IoError(io::Error, String),
    RawLoaderError(String),
    FITSErr(String),
    UnexpectedInput(String),
    ShapeError(ndarray::ShapeError, String),
}

impl fmt::Display for IOErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use crate::IOErr::*;

        match self {
            IoError(e, s) => write!(f, "I/O error! {}. This was caused by '{}'.", s, e),
            RawLoaderError(s) => write!(f, "RawLoader-related error! {}", s),
            FITSErr(s) => write!(f, "FITS-related error! {}", s),
            UnexpectedInput(s) => write!(f, "Unexpected input! {}", s),
            ShapeError(e, s) => write!(f, "Shape error! {}. This was caused by '{}'.", s, e),
        }
    }
}

impl Error for IOErr {
    fn description(&self) -> &str {
        "aflak_primitives::IOErr"
    }
}

/// Represent the successful result of an astrophysical computation.
pub type SuccessOut = cake::compute::SuccessOut<IOValue>;

lazy_static! {
    /// The exhaustive list of all staticly loaded astrophysical transforms.
    pub static ref TRANSFORMATIONS: Vec<cake::Transform<'static, IOValue, IOErr>> = {
        vec![
            cake_transform!(
                "Open FITS file from a Paths.",
                "01. Open File",
                1, 0, 0,
                open_fits<IOValue, IOErr>(path: Paths, n: Integer = 0) -> Fits {
                    let PATHS::FileList(path) = path;
                    vec![run_open_fits(path.to_vec(), *n)]
                }
            ),
            cake_transform!(
                "Open RAW file from a Paths.",
                "01. Open File",
                0, 1, 0,
                open_raw<IOValue, IOErr>(path: Paths, n: Integer = 0) -> Image {
                    let PATHS::FileList(path) = path;
                    vec![run_open_raw(path.to_vec(), *n)]
                }
            ),
            cake_transform!(
                "Extract dataset from FITS file.",
                "04. Extract part of data",
                1, 0, 0,
                fits_to_image<IOValue, IOErr>(fits: Fits, hdu_idx: Integer = 0, extension: Str = "".to_owned()) -> Image {
                    vec![run_fits_to_image(fits, *hdu_idx, extension)]
                }
            ),
            cake_transform!(
                "Range specification. Parameter: image, start, end.
Extract data where the 0th axis is from start to end",
                "04. Extract part of data",
                0, 1, 0,
                range_specification<IOValue, IOErr>(image: Image, start: Integer = 0, end: Integer = 1) -> Image {
                    vec![run_range_specification(image, *start, *end)]
                }
            ),
            cake_transform!(
                "Range specification. Parameter: image, start, end.
Extract data where the 1st axis is from start to end",
                "04. Extract part of data",
                0, 1, 0,
                range_specification_x<IOValue, IOErr>(image: Image, start: Integer = 0, end: Integer = 1) -> Image {
                    vec![run_range_specification_x(image, *start, *end)]
                }
            ),
            cake_transform!(
                "Range specification. Parameter: image, start, end.
Extract data where the 2nd axis is from start to end",
                "04. Extract part of data",
                0, 1, 0,
                range_specification_y<IOValue, IOErr>(image: Image, start: Integer = 0, end: Integer = 1) -> Image {
                    vec![run_range_specification_y(image, *start, *end)]
                }
            ),
            cake_transform!(
                "Image's min and max value. Parameter: image.
Compute v_min(first), v_max(second)",
                "04. Extract part of data",
                1, 0, 0,
                image_min_max<IOValue, IOErr>(image: Image) -> Float, Float {
                    let mut min = std::f32::MAX;
                    let mut max = std::f32::MIN;
                    let image_arr = image.scalar();

                    for i in image_arr {
                        min = min.min(*i);
                        max = max.max(*i);
                    }

                    vec![Ok(IOValue::Float(min)), Ok(IOValue::Float(max))]
                }
            ),
            cake_transform!(
                "Image's statistics. Parameter: image.
Compute mean(first), median(second), stdDev(third), background(forth)",
                "04. Extract part of data",
                1, 0, 0,
                compute_background<IOValue, IOErr>(image: Image, sigma_high: Float = 3.0, sigma_low: Float = 3.0, alpha: Float = 2.0) -> Float, Float, Float, Float {
                    let (mean, median, stddev, background) = run_compute_background(image, *sigma_high, *sigma_low, *alpha);
                    vec![Ok(IOValue::Float(mean)), Ok(IOValue::Float(median)), Ok(IOValue::Float(stddev)), Ok(IOValue::Float(background))]
                }
            ),
            cake_transform!(
                "Make a Float3 from 3 float values.",
                "02. Make new data",
                1, 0, 0,
                make_float3<IOValue, IOErr>(f1: Float = 0.0, f2: Float = 0.0, f3: Float = 0.0) -> Float3 {
                    vec![run_make_float3(*f1, *f2, *f3)]
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
                "02. Make new data",
                1, 0, 0,
                make_plane3d<IOValue, IOErr>(p0: Float3 = [0.0; 3], dir1: Float3 = [0.0, 0.0, 1.0], dir2: Float3 = [0.0, 1.0, 0.0], count1: Integer = 1, count2: Integer = 1) -> Map2dTo3dCoords {
                    vec![run_make_plane3d(p0, dir1, dir2, *count1, *count2)]
                }
            ),
            cake_transform!(
                "Ratio from bands' center wavelength.
Parameters: z(on-band's center wavelength), z1, z2(off-bands' centerwavelength) (z1 < z < z2).
Compute off_ratio = 1 - (z - z1) / (z2 - z1), off_ratio_2 = 1 - (z2 - z) / (z2 - z1)",
                "02. Make new data",
                1, 0, 0,
                ratio_from_bands<IOValue, IOErr>(z: Float, z1: Float, z2: Float) -> Float, Float {
                    if !(z1 < z && z < z2) {
                        use IOErr::UnexpectedInput;
                        let msg = format!(
                            "wrong magnitude correlation ({} < {} < {})",
                            z1, z, z2
                        );
                        vec![msg; 2].into_iter().map(|msg| Err(UnexpectedInput(msg))).collect()
                    } else {
                        let off_ratio_1 = (z - z1) / (z2 - z1);
                        let off_ratio_2 = 1.0 - off_ratio_1;
                        vec![Ok(IOValue::Float(off_ratio_2)), Ok(IOValue::Float(off_ratio_1))]
                    }
                }
            ),
            cake_transform!(
                "Create scatterplots from two image data.
Parameters: xaxis, yaxis.
The data sizes for xaxis and yaxis must be the same.",
                "02. Make new data",
                1, 0, 0,
                create_scatter<IOValue, IOErr>(xaxis: Image, yaxis: Image) -> Image{
                    vec![run_create_scatter(xaxis, yaxis)]
                }
            ),
            cake_transform!("Replace all values above or below a threshold in a image with NaN.
Takes two parameters: a threshold and a bool.
If bool value is checked, then replaces the values above the threshold with NaN, else replace the values below the threshold with NaN.",
                "05. Convert data",
                1, 0, 0,
                clip_image<IOValue, IOErr>(image: Image, ceiling_threshold: Float = 0.0, ceiling: Bool = false, floor_threshold: Float = 0.0, floor: Bool = false) -> Image {
                    vec![run_clip(image, *ceiling_threshold, *ceiling, *floor_threshold, *floor)]
                }
            ),
            cake_transform!("Estimate background and replace below background with NaN.
Parameters:sigma_high/sigma_low: Sigma clipping parameters, alpha: User preference parameter.",
                "05. Convert data",
                1, 0, 0,
                clip_background<IOValue, IOErr>(image: Image, sigma_high: Float = 3.0, sigma_low: Float = 3.0, alpha: Float = 2.0) -> Image {
                    vec![run_clip_background(image, *sigma_high, *sigma_low, *alpha)]
                }
            ),
            cake_transform!("Normalize image",
                "05. Convert data",
                1, 0, 0,
                normalize_image<IOValue, IOErr>(image: Image, min: Float = 0.0, max: Float = 1.0) -> Image {
                    let mut out = image.clone();
                    out.scalar_mut().par_iter_mut().for_each(|v| *v = (*v - min) / (max - min));
                    vec![Ok(IOValue::Image(out))]
                }
            ),
            cake_transform!("Replace all NaN values in image with the provided value.",
                "05. Convert data",
                1, 0, 0,
                replace_nan_image<IOValue, IOErr>(image: Image, placeholder: Float = 0.0) -> Image {
                    vec![run_replace_nan_image(image, *placeholder)]
                }
            ),
            cake_transform!(
                "Convert to log-scale. Parameter: image, a, v_min, v_max.
Compute y = log(ax + 1) / log(a)  (x = (value - v_min) / (v_max - v_min))",
                "05. Convert data",
                1, 0, 0,
                convert_to_logscale<IOValue, IOErr>(image: Image, a: Float = 1000.0, v_min: Float, v_max: Float) -> Image {
                    vec![run_convert_to_logscale(image, *a, *v_min, *v_max)]
                }
            ),
            cake_transform!(
                "Calculating log10 of input data.
Parameter: image.
Compute: log10(image)",
                "05. Convert data",
                1, 0, 0,
                log10<IOValue, IOErr>(image: Image) -> Image {
                    vec![run_log10(image)]
                }
            ),
            cake_transform!(
                "Negation. Parameter: image. Compute -image.",
                "05. Convert data",
                1, 0, 0,
                negation<IOValue, IOErr>(image: Image) -> Image {
                    vec![run_negation(image)]
                }
            ),
            cake_transform!(
                "Apply tone curve to image.",
                "05. Convert data",
                1, 0, 0,
                apply_tone_curve<IOValue, IOErr>(image: Image, tone_curve: ToneCurve) -> Image {
                    vec![run_apply_tone_curve(image, tone_curve.clone())]
                }
            ),
            cake_transform!(
                "Apply arcsinh stretch to image.",
                "05. Convert data",
                0, 1, 0,
                apply_arcsinh_stretch<IOValue, IOErr>(image: Image, beta: Float = 0.000000001) -> Image {
                    vec![run_apply_arcsinh_stretch(image, *beta)]
                }
            ),
            cake_transform!(
                "Change the visualization tag.
Currently supported tags: \"BPT\", \"color_image\"",
                "05. Convert data",
                1, 0, 0,
                change_tag<IOValue, IOErr>(image: Image, tag: Str = "".to_owned()) -> Image {
                    vec![run_change_tag(image, tag)]
                }
            ),
            cake_transform!(
                "Compose 2 images. Parameters: u, v, a, b.
Compute a*u + b*v.",
                "06. Calculate",
                1, 0, 0,
                linear_composition<IOValue, IOErr>(u: Image, v: Image, a: Float = 1.0, b: Float = 1.0) -> Image {
                    vec![run_linear_composition(u, v, *a, *b)]
                }
            ),
            cake_transform!(
                "Compose 2 images. Parameters: u, v, a, b.
Compute u^a * v^b.",
                "06. Calculate",
                1, 0, 0,
                image_multiplier<IOValue, IOErr>(u: Image, v: Image, a: Float = 1.0, b: Float = 1.0) -> Image {
                    vec![run_image_multiplier(u, v, *a, *b)]
                }
            ),
            cake_transform!(
                "Slice one frame of a n-dimensional dataset turning it into an (n-1)-dimensional dataset.",
                "07. Reduce dimension",
                1, 0, 0,
                slice_one_frame<IOValue, IOErr>(image: Image, frame: Integer = 0) -> Image {
                    vec![run_slice_one_frame(image, *frame)]
                }
            ),
            cake_transform!(
                "Slice an arbitrary plane through a 3D dataset and return the slice.",
                "07. Reduce dimension",
                0, 1, 0,
                slice_3d_to_2d<IOValue, IOErr>(image: Image, map: Map2dTo3dCoords) -> Image {
                    vec![run_slice_3d_to_2d(image, map)]
                }
            ),
            cake_transform!(
                "Extrude along the 0th axis (maybe wavelength).",
                "07. Reduce dimension",
                0, 1, 0,
                extrude<IOValue, IOErr>(image: Image, roi: Roi = roi::ROI::All) -> Image {
                    vec![run_extrude(image, roi)]
                }
            ),
            cake_transform!(
                "Extract waveform from image with the provided region of interest.",
                "07. Reduce dimension",
                1, 0, 0,
                extract_wave<IOValue, IOErr>(image: Image, roi: Roi = roi::ROI::All) -> Image {
                    vec![run_extract_wave(image, roi)]
                }
            ),
            cake_transform!(
                "Integral for Image. Parameters: a=start, b=end (a <= b).
Compute Sum[k, {a, b}]image[k]. image[k] is k-th slice of image.
Second output contains (a + b) / 2
Third output contains (b - a)
Note: indices for a and b start from 0",
"07. Reduce dimension",
                1, 0, 0,
                integral<IOValue, IOErr>(image: Image, start: Integer = 0, end: Integer = 1) -> Image, Float, Float {
                    let middle = (*start as f32 + *end as f32) / 2.0;
                    let width = *end as f32 - *start as f32;
                    vec![run_integral(image, *start, *end), Ok(IOValue::Float(middle)), Ok(IOValue::Float(width))]
                }
            ),
            cake_transform!(
                "Average for Image. Parameters: a=start, b=end (a <= b).
Compute (Sum[k, {a, b}]image[k]) / (b - a). image[k] is k-th slice of image.
Second output contains (a + b) / 2
Third output contains (b - a)
Note: indices for a and b start from 0",
"07. Reduce dimension",
                1, 0, 0,
                average<IOValue, IOErr>(image: Image, start: Integer = 0, end: Integer = 1) -> Image, Float, Float {
                    let middle = (*start as f32 + *end as f32) / 2.0;
                    let width = *end as f32 - *start as f32;
                    vec![run_average(image, *start, *end), Ok(IOValue::Float(middle)), Ok(IOValue::Float(width))]
                }
            ),
            cake_transform!(
                "Variance for Image. Parameters: a=start, b=end (a <= b).",
                "07. Reduce dimension",
                1, 0, 0,
                variance<IOValue, IOErr>(image: Image, start: Integer = 0, end: Integer = 1) -> Image{
                    vec![run_variance(image, *start, *end)]
                }
            ),
            cake_transform!(
                "Stddev for Image. Parameters: a=start, b=end (a <= b).",
                "07. Reduce dimension",
                1, 0, 0,
                stddev<IOValue, IOErr>(image: Image, start: Integer = 0, end: Integer = 1) -> Image{
                    vec![run_stddev(image, *start, *end)]
                }
            ),
            cake_transform!(
                "Median for Image. Parameters: a=start, b=end (a <= b).",
                "07. Reduce dimension",
                1, 0, 0,
                median<IOValue, IOErr>(image: Image, start: Integer = 0, end: Integer = 1) -> Image{
                    vec![run_median(image, *start, *end)]
                }
            ),
            cake_transform!(
                "Extract min/max wavelength value of each pixel.
Parameter: image, start, end, is_min (start <= end)
Output argmax/argmin map of flux; wavelength
Second output contains max/min flux map
Note: output wavelength values are discrete. indices for start and end start from 0",
"08. Reduce dimension (index)",
                0, 1, 0,
                extract_argmin_max_wavelength<IOValue, IOErr>(image: Image, start: Integer = 0, end: Integer = 1, is_min: Bool = false) -> Image, Image {
                    vec![run_argminmax(image, *start, *end, *is_min), run_minmax(image, *start, *end, *is_min)]
                }
            ),
            cake_transform!(
                "peak-based_wavelength_range.
Parameter: image, start, end, range, is_min (start <= end)
Output [argmax - range, argmax + range] [argmin - range, argmin + range] map of flux; wavelength
Note: output wavelength values are discrete. indices for start and end start from 0",
"08. Reduce dimension (index)",
                0, 1, 0,
                peak_based_wavelength_range<IOValue, IOErr>(image: Image, start: Integer = 0, end: Integer = 1, range: Integer = 1, is_min: Bool = false) -> Image, Image {
                    vec![run_create_argmap(image, *start, *end, -*range, *is_min, false), run_create_argmap(image, *start, *end, *range, *is_min, false)]
                }
            ),
            cake_transform!(
                "Extract centrobaric wavelength value of each pixel.
Parameter: image (which has wavelength value w_i and flux f_i), start, end
Compute Sum[k, (start, end)](f_k * w_k) / Sum(k, (start, end)(f_k))
Note: indices for start and end start from 0",
"08. Reduce dimension (index)",
                0, 1, 0,
                extract_centrobaric_wavelength<IOValue, IOErr>(image: Image, start: Integer = 0, end: Integer = 1) -> Image {
                    vec![run_centroid(image, *start, *end)]
                }
            ),
            cake_transform!(
                "Extract centrobaric wavelength value of each pixel with mask.
Parameter: image (which has wavelength value w_i and flux f_i), start_mask, end_mask
Compute Sum[k, (start, end)](f_k * w_k) / Sum(k, (start, end)(f_k))",
"08. Reduce dimension (index)",
                0, 1, 0,
                extract_centrobaric_wavelength_with_mask<IOValue, IOErr>(image: Image, start_mask: Image, end_mask: Image) -> Image {
                    vec![run_centroid_with_mask(image, start_mask, end_mask)]
                }
            ),
            cake_transform!(
                "Gaussian. Parameter: image, start, end
    Compute mean, when the (x, y) is fitted as y = A * exp(-(x - mean) ^ 2 / (2 * sigma ^ 2))",
    "08. Reduce dimension (index)",
                    0, 1, 0,
                    gaussian<IOValue, IOErr>(image: Image, start: Integer = 0, end: Integer = 1) -> Image {
                        vec![run_gaussian_mean(image, *start, *end)]
                    }
                ),
                cake_transform!(
                    "Gaussian with mask. Parameter: image, start_mask, end_mask
    Compute mean, when the (x, y) is fitted as y = A * exp(-(x - mean) ^ 2 / (2 * sigma ^ 2))",
    "08. Reduce dimension (index)",
                    0, 1, 0,
                    gaussian_with_mask<IOValue, IOErr>(image: Image, start_mask: Image, end_mask: Image) -> Image {
                        vec![run_gaussian_mean_with_mask(image, start_mask, end_mask)]
                    }
                ),
            cake_transform!(
                "Create Equivalent-Width map from off-band and on-band.
Parameters i_off, i_on, onband-width, min, is_emission.
Compute value = (i1 - i2) * fl / i1 (if is_emission is true, the sign of this value turns over).
if value > max, value changes to 0.",
                "10. Create astronomy-specific map",
                0, 1, 0,
                create_equivalent_width<IOValue, IOErr>(i_off: Image, i_on: Image, fl: Float = 1.0, max: Float = ::std::f32::INFINITY, is_emission: Bool = false) -> Image {
                    vec![run_create_equivalent_width(i_off, i_on, *fl, *max, *is_emission)]
                }
            ),
            cake_transform!(
                "Create velocity field map
Parameter: image (which has wavelength value w_i in each pixel), representative wavelength w_0
Compute Velocity v = c * (w_i - w_0) / w_0   (c = 3e5 [km/s])",
"10. Create astronomy-specific map",
                1, 0, 0,
                create_velocity_field_map<IOValue, IOErr>(image: Image, w_0: Float = 0.0) -> Image {
                    let c = Unit::Custom("km/s".to_owned()).new(3e5 as f32);
                    let w_0 = Unit::Custom("Ang".to_owned()).new(w_0.to_owned());
                    let result = (image.array() - w_0.clone()) * c / w_0;
                    let original_meta = image.meta();
                    let original_visualization = image.tag();
                    let original_topology = image.topology();

                    let result = WcsArray::new(original_meta.to_owned(), result, original_visualization.to_owned(), original_topology.to_owned());

                    vec![Ok(IOValue::Image(result))]
                }
            ),
            cake_transform!(
                "Create Emission line map from off-band and on-band.
Parameters i_off, i_on, onband-width, min, is_emission.
Compute value = (i1 - i2) * fl (if is_emission is true, the sign of this value turns over).Integral
if value > max, value changes to 0.",
"10. Create astronomy-specific map",
                0, 1, 0,
                create_emission_line_map<IOValue, IOErr>(i_off: Image, i_on: Image, fl: Float = 1.0, max: Float = ::std::f32::INFINITY, is_emission: Bool = false) -> Image {
                    vec![run_create_emission_line_map(i_off, i_on, *fl, *max, *is_emission)]
                }
            ),
            cake_transform!(
                "Generate hsv channel from rgb image",
                "09. Reduce dimension (color image)",
                1, 0, 0,
                color_image_to_hsv<IOValue, IOErr>(image: Image) -> Image {
                    vec![run_color_image_to_hsv(image)]
                }
            ),
            cake_transform!(
                "Generate color image from rgb channels",
                "03. Make new data (color)",
                1, 0, 0,
                color_image_from_rgb<IOValue, IOErr>(image_r: Image, image_g: Image, image_b: Image) -> Image {
                    vec![run_generate_color_image_from_channel(image_r, image_g, image_b)]
                }
            ),
            cake_transform!(
                "Downsample the image. The width and height of size will be 1/n",
                "02. Make new data",
                1, 0, 0,
                down_sampling<IOValue, IOErr>(image: Image, n: Integer = 1) -> Image {
                    vec![run_down_sampling(image, *n)]
                }
            ),
            cake_transform!(
                "Histogram Transformation.",
                "05. Convert data",
                0, 1, 0,
                histogram_transformation<IOValue, IOErr>(image: Image, parameters: Float3 = [0.0, 0.5, 1.0], _dynamic_range: Float2 = [0.0, 1.0]) -> Image {
                    let mut min = std::f32::MAX;
                    let mut max = std::f32::MIN;
                    let image_arr = image.scalar();

                    for i in image_arr {
                        min = min.min(*i);
                        max = max.max(*i);
                    }
                    let mut out = image.clone();
                    out.scalar_mut().par_iter_mut().for_each(|v| {
                        let point = (*v - min) / (max - min);
                        let x = (point - parameters[0]) / (parameters[2] - parameters[0]);
                        let c = if x > 1.0 {
                            1.0
                        } else if x < 0.0 {
                            0.0
                        } else {
                            let m = parameters[1];
                            (m - 1.0) * x / ((2.0 * m - 1.0) * x - m)
                        };
                        *v = c * (max - min) + min;
                    });
                    vec![Ok(IOValue::Image(out))]
                }
            ),
            cake_transform!(
                "Histogram Transformation (color).",
                "05. Convert data",
                0, 1, 0,
                histogram_transformation_rgb<IOValue, IOErr>(image: Image, parameters: Float3x3 = [[0.0, 0.5, 1.0], [0.0, 0.5, 1.0], [0.0, 0.5, 1.0]], _dynamic_range: Float2 = [0.0, 1.0]) -> Image {
                    let mut mins = [std::f32::MAX; 3];
                    let mut mids = [std::f32::NAN; 3];
                    let mut maxs = [std::f32::MIN; 3];
                    let dim = image.scalar().dim();
                    let dim = dim.as_array_view();
                    let tag = image.tag();
                    for (c, slice) in image.scalar().axis_iter(Axis(0)).enumerate() {
                        for i in slice {
                            mins[c] = mins[c].min(*i);
                            maxs[c] = maxs[c].max(*i);
                            mids[c] = parameters[c][1];
                        }
                    }
                    let mut data = Vec::with_capacity(dim[0] * dim[1] * dim[2]);
                    for (ch, slice) in image.scalar().axis_iter(Axis(0)).enumerate() {
                        for v in slice.iter() {
                            let point = (*v - mins[ch]) / (maxs[ch] - mins[ch]);
                            let x = (point - parameters[ch][0]) / (parameters[ch][2] - parameters[ch][0]);
                            let c = if x > 1.0 {
                                1.0
                            } else if x < 0.0 {
                                0.0
                            } else {
                                (mids[ch] - 1.0) * x / ((2.0 * mids[ch] - 1.0) * x - mids[ch])
                            };
                            data.push(c * (maxs[ch] - mins[ch]) + mins[ch]);
                        }
                    }
                    let img = Array::from_shape_vec((dim[0], dim[1], dim[2]), data).unwrap();
                    vec![Ok(IOValue::Image(WcsArray::from_array_and_tag(Dimensioned::new(
                        img.into_dyn(),
                        Unit::None,
                    ), tag.to_owned())))]
                }
            ),
            cake_transform!(
                "Dynamic Background Extraction.",
                "05. Convert data",
                0, 1, 0,
                dynamic_background_extraction<IOValue, IOErr>(image: Image, points: Roi, function_degree: Integer = 1) -> Image, Image {
                    let tag = image.tag();
                    let dim = image.scalar().dim();
                    let dim = dim.as_array_view();
                    let funcdeg_plus_one = (*function_degree + 1) as usize;
                    let params_right_num = funcdeg_plus_one * funcdeg_plus_one;
                    let mut params_right = Vec::<f32>::with_capacity(params_right_num * dim[0]);
                    params_right.resize(params_right_num * dim[0], 0.0);
                    let mut params_right = Array::from_vec(params_right).into_shape((dim[0], funcdeg_plus_one, funcdeg_plus_one)).unwrap();
                    let params_left_num = params_right_num * params_right_num;
                    let mut params_left = Vec::<f32>::with_capacity(params_left_num * dim[0]);
                    params_left.resize(params_left_num * dim[0], 0.0);
                    let mut params_left = Array::from_vec(params_left)
                        .into_shape((
                            dim[0],
                            funcdeg_plus_one,
                            funcdeg_plus_one,
                            funcdeg_plus_one,
                            funcdeg_plus_one,
                        ))
                        .unwrap();
                    let mut model = image.scalar().clone();
                    let mut out = image.scalar().clone();
                    let mut compute_failure = false;
                    for i in 0..dim[0] {
                        for takes_y in 0..funcdeg_plus_one {
                            for takes_x in 0..funcdeg_plus_one {
                                for inner_takes_y in 0..funcdeg_plus_one {
                                    for inner_takes_x in 0..funcdeg_plus_one {
                                        let data = points.filter_upside_down(image.scalar().slice(s![i, .., ..]));
                                        for ((x, y), _) in data {
                                            let x = x as f32 / dim[2] as f32;
                                            let y = y as f32 / dim[1] as f32;
                                            params_left[[i, takes_y, takes_x, inner_takes_y, inner_takes_x]] += 1.0
                                                * (x as f32).powf((takes_x + inner_takes_x) as f32)
                                                * (y as f32).powf((takes_y + inner_takes_y) as f32);
                                        }
                                    }
                                }
                                let data = points.filter_upside_down(image.scalar().slice(s![i, .., ..]));
                                for ((x, y), _) in data {
                                    let mut boxdata = Vec::new();
                                    for xx in (x - 10).max(0)..(x + 10).min(dim[2]) {
                                        for yy in (y - 10).max(0)..(y + 10).min(dim[1]) {
                                            boxdata.push(model[[i, yy, xx]]);
                                        }
                                    }

                                    boxdata.sort_by(|a, b| a.partial_cmp(b).unwrap());
                                    let mid = boxdata.len() / 2;
                                    let med = if boxdata.len() % 2 == 0 {
                                        (boxdata[mid] + boxdata[mid - 1]) / 2.0
                                    } else {
                                        boxdata[mid]
                                    };
                                    boxdata.clear();
                                    let x = x as f32 / dim[2] as f32;
                                    let y = y as f32 / dim[1] as f32;
                                    params_right[[i, takes_y, takes_x]] +=
                                        med * (x as f32).powf(takes_x as f32) * (y as f32).powf(takes_y as f32);
                                }
                            }
                        }
                        let params_left_vec = params_left.index_axis(Axis(0), i).to_owned().into_raw_vec();
                        let params_right_vec = params_right.index_axis(Axis(0), i).to_owned().into_raw_vec();
                        let a = DMatrix::from_vec(params_right_num, params_right_num, params_left_vec);
                        let b = DVector::from(params_right_vec.clone());
                        let decomp = a.lu();
                        let x = decomp.solve(&b);
                        let mut sol = DVector::from(params_right_vec);
                        match x {
                            Some(vector) => sol = vector,
                            None => {
                                println!("failure");
                                compute_failure = true;
                            },
                        };
                        if compute_failure {
                            break;
                        }
                        for y in 0..dim[1] {
                            for x in 0..dim[2] {
                                let xx = x as f32 / dim[2] as f32;
                                let yy = y as f32 / dim[1] as f32;
                                model[[i, y, x]] = 0.0;
                                for takes_x in 0..funcdeg_plus_one {
                                    for takes_y in 0..funcdeg_plus_one {
                                        model[[i, y, x]] += sol[takes_y * funcdeg_plus_one + takes_x]
                                            * (xx.powf(takes_x as f32))
                                            * (yy.powf(takes_y as f32));
                                    }
                                }
                                out[[i, y, x]] -= model[[i, y, x]];
                            }
                        }
                    }
                    if !compute_failure {
                        vec![Ok(IOValue::Image(WcsArray::from_array_and_tag(Dimensioned::new(
                            out,
                            Unit::None,
                        ), tag.to_owned()))), Ok(IOValue::Image(WcsArray::from_array_and_tag(Dimensioned::new(
                            model,
                            Unit::None,
                        ), tag.to_owned())))]
                    } else {
                        vec![Err(IOErr::UnexpectedInput("Linear algebra failed.".to_string())), Err(IOErr::UnexpectedInput("Linear algebra failed.".to_string()))]
                    }
                }
            ),
            cake_transform!(
                "TTK Test node, call hello in TTK",
                "10. Topological Analysis",
                1, 0, 0,
                ttk_hello<IOValue, IOErr>(image: Image) -> Image {
                    unsafe {
                        let mut my_ttk = Ttk_rs::new();
                        my_ttk.ttk_helloworld();
                    }
                    vec![Ok(IOValue::Image(image.clone()))]
                }
            ),
            cake_transform!(
                "TTKTest use C++",
                "10. Topological Analysis in C++",
                1, 0, 0,
                ttk_persistence_pairs<IOValue, IOErr>(image: Image) -> PersistencePairs {
                    vec![run_ttk_persistence_pairs(image)]
                }
            ),
            cake_transform!(
                "TTKTest use C++",
                "10. Topological Analysis in C++",
                1, 0, 0,
                ttk_persistence_pairs_3d<IOValue, IOErr>(image: Image) -> PersistencePairs {
                    vec![run_ttk_persistence_pairs_3d(image)]
                }
            ),
            cake_transform!(
                "TTKTest",
                "10. Topological Analysis in C++",
                1, 0, 0,
                select_the_most_pairs<IOValue, IOErr>(pp: PersistencePairs, filter: Float = 0.05) -> PersistencePairs {
                    vec![run_select_the_most_pairs(pp.clone(), *filter)]
                }
            ),
            cake_transform!(
                "TTKTest",
                "10. Topological Analysis in C++",
                1, 0, 0,
                select_the_most_pairs_using_sigma<IOValue, IOErr>(pp: PersistencePairs, kappa: Float = 1.0) -> PersistencePairs {
                    vec![run_select_the_most_pairs_using_sigma(pp.clone(), *kappa)]
                }
            ),
            cake_transform!(
                "TTKTest",
                "10. Topological Analysis in C++",
                1, 0, 0,
                ttk_simplification<IOValue, IOErr>(image: Image, pp: PersistencePairs) -> Image {
                    vec![run_ttk_simplification(image, pp.clone())]
                }
            ),
            cake_transform!(
                "TTKTest",
                "10. Topological Analysis in C++",
                1, 0, 0,
                ttk_simplification_3d<IOValue, IOErr>(image: Image, pp: PersistencePairs) -> Image {
                    vec![run_ttk_simplification_3d(image, pp.clone())]
                }
            ),
            cake_transform!(
                "TTKTest",
                "10. Topological Analysis in C++",
                1, 0, 0,
                ttk_test_ftr<IOValue, IOErr>(image: Image) -> Image {
                    vec![run_ttk_ftr(image)]
                }
            ),
            cake_transform!(
                "TTKTest",
                "10. Topological Analysis in C++",
                1, 0, 0,
                ttk_test_kl_map<IOValue, IOErr>(image: Image) -> Image {
                    vec![run_ttk_create_kappa_lambda_map(image)]
                }
            ),
            cake_transform!(
                "TTKTest",
                "10. Topological Analysis in C++",
                1, 0, 0,
                ttk_test_tindexes_map<IOValue, IOErr>(image: Image) -> Image {
                    vec![run_ttk_create_tindexes_map(image)]
                }
            ),
            cake_transform!(
                "TTKTest",
                "10. Topological Analysis in C++",
                1, 0, 0,
                lab_to_rgb<IOValue, IOErr>(image: Image) -> Image {
                    vec![run_lab_to_rgb(image)]
                }
            )
        ]
    };
}

fn run_ttk_persistence_pairs(image: &WcsArray) -> Result<IOValue, IOErr> {
    dim_is!(image, 2)?;
    let image_val = image.scalar();
    let dim = image_val.dim();
    let dim = dim.as_array_view();
    let mut data = Vec::new();
    for i in image_val {
        data.push(*i);
    }
    let data_ptr = data.as_mut_ptr();
    let mut birth = Vec::<i32>::with_capacity(image_val.len());
    let mut death = Vec::<i32>::with_capacity(image_val.len());
    let birth_ptr = birth.as_mut_ptr();
    let death_ptr = death.as_mut_ptr();
    let mut act_pair = Vec::new();
    let mut len = 0;
    unsafe {
        let mut my_ttk = Ttk_rs::new();
        my_ttk.compute_persistence_pairs(
            data_ptr,
            data.len() as u32,
            dim[0] as u32,
            dim[1] as u32,
            birth_ptr,
            death_ptr,
            &mut len,
        );
        let birth = slice::from_raw_parts(birth_ptr, len as usize).to_vec();
        let death = slice::from_raw_parts(death_ptr, len as usize).to_vec();
        for i in 0..len {
            act_pair.push((
                *(birth.get(i as usize).unwrap()),
                *(death.get(i as usize).unwrap()),
            ));
        }
    }
    let mut pp = Vec::<(i32, i32, f32, f32)>::new();
    for (i, j) in act_pair {
        pp.push((
            i,
            j,
            *data.get(i as usize).unwrap(),
            *data.get(j as usize).unwrap(),
        ));
    }
    Ok(IOValue::PersistencePairs(PersistencePairs::Pairs(pp)))
}

fn run_ttk_persistence_pairs_3d(image: &WcsArray) -> Result<IOValue, IOErr> {
    dim_is!(image, 3)?;
    let image_val = image.scalar();
    let dim = image_val.dim();
    let dim = dim.as_array_view();
    let mut data = Vec::new();
    for i in image_val {
        data.push(*i);
    }
    let data_ptr = data.as_mut_ptr();
    let mut birth = Vec::<i32>::with_capacity(image_val.len());
    let mut death = Vec::<i32>::with_capacity(image_val.len());
    let birth_ptr = birth.as_mut_ptr();
    let death_ptr = death.as_mut_ptr();
    let mut act_pair = Vec::new();
    let mut len = 0;
    unsafe {
        let mut my_ttk = Ttk_rs::new();
        my_ttk.compute_persistence_pairs_3d(
            data_ptr,
            data.len() as u32,
            dim[1] as u32,
            dim[2] as u32,
            dim[0] as u32,
            birth_ptr,
            death_ptr,
            &mut len,
        );
        let birth = slice::from_raw_parts(birth_ptr, len as usize).to_vec();
        let death = slice::from_raw_parts(death_ptr, len as usize).to_vec();
        for i in 0..len {
            act_pair.push((
                *(birth.get(i as usize).unwrap()),
                *(death.get(i as usize).unwrap()),
            ));
        }
    }
    let mut pp = Vec::<(i32, i32, f32, f32)>::new();
    for (i, j) in act_pair {
        let vi = *data.get(i as usize).unwrap();
        let vj = *data.get(j as usize).unwrap();
        if !vi.is_nan() && !vj.is_nan() {
            pp.push((i, j, vi, vj));
        }
    }
    Ok(IOValue::PersistencePairs(PersistencePairs::Pairs(pp)))
}

fn run_select_the_most_pairs(pp: PersistencePairs, filter: f32) -> Result<IOValue, IOErr> {
    let PersistencePairs::Pairs(mut data) = pp;
    data.retain(|d| d.3 - d.2 > filter);
    Ok(IOValue::PersistencePairs(PersistencePairs::Pairs(data)))
}

fn run_select_the_most_pairs_using_sigma(
    pp: PersistencePairs,
    kappa: f32,
) -> Result<IOValue, IOErr> {
    let PersistencePairs::Pairs(mut data) = pp;
    let mut average = 0.0;
    for d in &data {
        average += d.3 - d.2;
    }
    average /= data.len() as f32;
    let mut sigma = 0.0;
    for d in &data {
        let xi = d.3 - d.2;
        sigma += (xi - average) * (xi - average);
    }
    sigma /= data.len() as f32;
    sigma = sigma.sqrt();
    data.retain(|d| {
        d.3 - d.2 > average
            && ((d.3 - d.2) - average) * ((d.3 - d.2) - average) > kappa * kappa * sigma * sigma
    });
    Ok(IOValue::PersistencePairs(PersistencePairs::Pairs(data)))
}

fn run_ttk_simplification(image: &WcsArray, pp: PersistencePairs) -> Result<IOValue, IOErr> {
    dim_is!(image, 2)?;
    let mut out = image.clone();
    let image_val = image.scalar();
    let dim = image_val.dim();
    let dim = dim.as_array_view();
    let mut data = Vec::new();
    let mut authorized_birth = Vec::new();
    let mut authorized_death = Vec::new();
    let PersistencePairs::Pairs(pairs) = pp;
    for i in image_val {
        data.push(*i);
    }
    for (b, d, _, _) in &pairs {
        authorized_birth.push(*b);
        authorized_death.push(*d);
    }
    let data_ptr = data.as_mut_ptr();
    let authorized_birth_ptr = authorized_birth.as_mut_ptr();
    let authorized_death_ptr = authorized_death.as_mut_ptr();

    //critical_points
    let mut cp_len = 0;
    const MAX_CRITICAL_POINTS: usize = 1024;
    let mut cp_point_types = Vec::<u32>::with_capacity(MAX_CRITICAL_POINTS);
    let mut cp_coordx = Vec::<f32>::with_capacity(MAX_CRITICAL_POINTS);
    let mut cp_coordy = Vec::<f32>::with_capacity(MAX_CRITICAL_POINTS);
    let mut cp_value = Vec::<f32>::with_capacity(MAX_CRITICAL_POINTS);
    let mut cp_cellid = Vec::<u32>::with_capacity(MAX_CRITICAL_POINTS);
    let mut cp_pl_vertex_identifier = Vec::<u32>::with_capacity(MAX_CRITICAL_POINTS);
    let mut cp_manifold_size = Vec::<u32>::with_capacity(MAX_CRITICAL_POINTS);

    let cp_point_types_ptr = cp_point_types.as_mut_ptr();
    let cp_coordx_ptr = cp_coordx.as_mut_ptr();
    let cp_coordy_ptr = cp_coordy.as_mut_ptr();
    let cp_value_ptr = cp_value.as_mut_ptr();
    let cp_cellid_ptr = cp_cellid.as_mut_ptr();
    let cp_pl_vertex_identifier_ptr = cp_pl_vertex_identifier.as_mut_ptr();
    let cp_manifold_size_ptr = cp_manifold_size.as_mut_ptr();

    //separatrices1_points
    let mut sp_len = 0;
    const MAX_SEPARATRICES1_POINTS: usize = 32768;
    let mut sp_id = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_POINTS);
    let mut sp_coordx = Vec::<f32>::with_capacity(MAX_SEPARATRICES1_POINTS);
    let mut sp_coordy = Vec::<f32>::with_capacity(MAX_SEPARATRICES1_POINTS);
    let mut sp_point_type = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_POINTS);
    let mut sp_cellid = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_POINTS);

    let sp_id_ptr = sp_id.as_mut_ptr();
    let sp_coordx_ptr = sp_coordx.as_mut_ptr();
    let sp_coordy_ptr = sp_coordy.as_mut_ptr();
    let sp_point_type_ptr = sp_point_type.as_mut_ptr();
    let sp_cellid_ptr = sp_cellid.as_mut_ptr();

    //separatrices1_cells
    let mut sc_len = 0;
    const MAX_SEPARATRICES1_CELLS: usize = 32768;
    let mut sc_id = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_source = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_dest = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_connectivity_s = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_connectivity_d = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_separatrix_id = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_separatrix_type = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_f_maxima = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_f_minima = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_f_diff = Vec::<f32>::with_capacity(MAX_SEPARATRICES1_CELLS);

    let sc_id_ptr = sc_id.as_mut_ptr();
    let sc_source_ptr = sc_source.as_mut_ptr();
    let sc_dest_ptr = sc_dest.as_mut_ptr();
    let sc_connectivity_s_ptr = sc_connectivity_s.as_mut_ptr();
    let sc_connectivity_d_ptr = sc_connectivity_d.as_mut_ptr();
    let sc_separatrix_id_ptr = sc_separatrix_id.as_mut_ptr();
    let sc_separatrix_type_ptr = sc_separatrix_type.as_mut_ptr();
    let sc_f_maxima_ptr = sc_f_maxima.as_mut_ptr();
    let sc_f_minima_ptr = sc_f_minima.as_mut_ptr();
    let sc_f_diff_ptr = sc_f_diff.as_mut_ptr();

    unsafe {
        let mut my_ttk = Ttk_rs::new();
        my_ttk.simplification(
            data_ptr,
            data.len() as u32,
            dim[1] as u32,
            dim[0] as u32,
            authorized_birth_ptr,
            authorized_death_ptr,
            pairs.len() as u32,
            cp_point_types_ptr,
            cp_coordx_ptr,
            cp_coordy_ptr,
            cp_value_ptr,
            cp_cellid_ptr,
            cp_pl_vertex_identifier_ptr,
            cp_manifold_size_ptr,
            &mut cp_len,
            sp_id_ptr,
            sp_coordx_ptr,
            sp_coordy_ptr,
            sp_point_type_ptr,
            sp_cellid_ptr,
            &mut sp_len,
            sc_id_ptr,
            sc_source_ptr,
            sc_dest_ptr,
            sc_connectivity_s_ptr,
            sc_connectivity_d_ptr,
            sc_separatrix_id_ptr,
            sc_separatrix_type_ptr,
            sc_f_maxima_ptr,
            sc_f_minima_ptr,
            sc_f_diff_ptr,
            &mut sc_len,
        );
        println!("After Simplification Rust part");
        println!(
            "cp_len: {}, MAX_CRITICAL_POINTS: {}",
            cp_len, MAX_CRITICAL_POINTS
        );
        if cp_len >= MAX_CRITICAL_POINTS as u32 {
            println!("maybe segfault!");
        }
        let cp_point_types = slice::from_raw_parts(cp_point_types_ptr, cp_len as usize).to_vec();
        let cp_coordx = slice::from_raw_parts(cp_coordx_ptr, cp_len as usize).to_vec();
        let cp_coordy = slice::from_raw_parts(cp_coordy_ptr, cp_len as usize).to_vec();
        let cp_value = slice::from_raw_parts(cp_value_ptr, cp_len as usize).to_vec();
        let cp_cellid = slice::from_raw_parts(cp_cellid_ptr, cp_len as usize).to_vec();
        let cp_pl_vertex_identifier =
            slice::from_raw_parts(cp_pl_vertex_identifier_ptr, cp_len as usize).to_vec();
        let cp_manifold_size =
            slice::from_raw_parts(cp_manifold_size_ptr, cp_len as usize).to_vec();

        println!(
            "sp_len: {}, MAX_SEPARATRICES1_POINTS: {}",
            sp_len, MAX_SEPARATRICES1_POINTS
        );
        if sp_len >= MAX_SEPARATRICES1_POINTS as u32 {
            println!("maybe segfault!");
        }

        let sp_id = slice::from_raw_parts(sp_id_ptr, sp_len as usize).to_vec();
        let sp_coordx = slice::from_raw_parts(sp_coordx_ptr, sp_len as usize).to_vec();
        let sp_coordy = slice::from_raw_parts(sp_coordy_ptr, sp_len as usize).to_vec();
        let sp_point_type = slice::from_raw_parts(sp_point_type_ptr, sp_len as usize).to_vec();
        let sp_cellid = slice::from_raw_parts(sp_cellid_ptr, sp_len as usize).to_vec();

        println!(
            "sc_len: {}, MAX_SEPARATRICES1_CELLS: {}",
            sc_len, MAX_SEPARATRICES1_CELLS
        );
        if sc_len >= MAX_SEPARATRICES1_CELLS as u32 {
            println!("maybe segfault!");
        }

        let sc_id = slice::from_raw_parts(sc_id_ptr, sc_len as usize).to_vec();
        let sc_source = slice::from_raw_parts(sc_source_ptr, sc_len as usize).to_vec();
        let sc_dest = slice::from_raw_parts(sc_dest_ptr, sc_len as usize).to_vec();
        let sc_connectivity_s =
            slice::from_raw_parts(sc_connectivity_s_ptr, sc_len as usize).to_vec();
        let sc_connectivity_d =
            slice::from_raw_parts(sc_connectivity_d_ptr, sc_len as usize).to_vec();
        let sc_separatrix_id =
            slice::from_raw_parts(sc_separatrix_id_ptr, sc_len as usize).to_vec();
        let sc_separatrix_type =
            slice::from_raw_parts(sc_separatrix_type_ptr, sc_len as usize).to_vec();
        let sc_f_maxima = slice::from_raw_parts(sc_f_maxima_ptr, sc_len as usize).to_vec();
        let sc_f_minima = slice::from_raw_parts(sc_f_minima_ptr, sc_len as usize).to_vec();
        let sc_f_diff = slice::from_raw_parts(sc_f_diff_ptr, sc_len as usize).to_vec();
        println!("After Read from_raw_parts");
        let mut critical_points = Vec::new();
        let mut separatrices1_points = Vec::new();
        let mut separatrices1_cells = Vec::new();
        println!("Compute CP");
        for i in 0..cp_len as usize {
            let cp = CriticalPoints::new(
                cp_point_types[i] as usize,
                (cp_coordx[i], cp_coordy[i], 0.0),
                cp_value[i],
                cp_cellid[i] as usize,
                cp_pl_vertex_identifier[i] as usize,
                cp_manifold_size[i] as usize,
            );
            critical_points.push(cp);
        }
        println!("Compute SP");
        for i in 0..sp_len as usize {
            let sp = Separatrices1Point::new(
                sp_id[i] as usize,
                (sp_coordx[i], sp_coordy[i], 0.0),
                sp_point_type[i] as usize,
                sp_cellid[i] as usize,
            );
            separatrices1_points.push(sp);
        }
        println!("Compute SC");
        for i in 0..sc_len as usize {
            let sc = Separatrices1Cell::new(
                sc_id[i] as usize,
                sc_source[i] as usize,
                sc_dest[i] as usize,
                (sc_connectivity_s[i] as usize, sc_connectivity_d[i] as usize),
                sc_separatrix_id[i] as usize,
                sc_separatrix_type[i] as usize,
                sc_f_maxima[i] as usize,
                sc_f_minima[i] as usize,
                sc_f_diff[i],
            );
            separatrices1_cells.push(sc);
        }
        let topology = Topology::new(critical_points, separatrices1_points, separatrices1_cells);
        out.set_topology(Some(topology));
        println!("All done!");
    }
    Ok(IOValue::Image(out))
}

fn run_ttk_simplification_3d(image: &WcsArray, pp: PersistencePairs) -> Result<IOValue, IOErr> {
    dim_is!(image, 3)?;
    let mut out = image.clone();
    let image_val = image.scalar();
    let dim = image_val.dim();
    let dim = dim.as_array_view();
    let mut data = Vec::new();
    let mut authorized_birth = Vec::new();
    let mut authorized_death = Vec::new();
    let PersistencePairs::Pairs(pairs) = pp;
    for i in image_val {
        data.push(*i);
    }
    for (b, d, _, _) in &pairs {
        authorized_birth.push(*b);
        authorized_death.push(*d);
    }
    let data_ptr = data.as_mut_ptr();
    let authorized_birth_ptr = authorized_birth.as_mut_ptr();
    let authorized_death_ptr = authorized_death.as_mut_ptr();

    //critical_points
    let mut cp_len = 0;
    const MAX_CRITICAL_POINTS: usize = 131072;
    let mut cp_point_types = Vec::<u32>::with_capacity(MAX_CRITICAL_POINTS);
    let mut cp_coordx = Vec::<f32>::with_capacity(MAX_CRITICAL_POINTS);
    let mut cp_coordy = Vec::<f32>::with_capacity(MAX_CRITICAL_POINTS);
    let mut cp_coordz = Vec::<f32>::with_capacity(MAX_CRITICAL_POINTS);
    let mut cp_value = Vec::<f32>::with_capacity(MAX_CRITICAL_POINTS);
    let mut cp_cellid = Vec::<u32>::with_capacity(MAX_CRITICAL_POINTS);
    let mut cp_pl_vertex_identifier = Vec::<u32>::with_capacity(MAX_CRITICAL_POINTS);
    let mut cp_manifold_size = Vec::<u32>::with_capacity(MAX_CRITICAL_POINTS);

    let cp_point_types_ptr = cp_point_types.as_mut_ptr();
    let cp_coordx_ptr = cp_coordx.as_mut_ptr();
    let cp_coordy_ptr = cp_coordy.as_mut_ptr();
    let cp_coordz_ptr = cp_coordz.as_mut_ptr();
    let cp_value_ptr = cp_value.as_mut_ptr();
    let cp_cellid_ptr = cp_cellid.as_mut_ptr();
    let cp_pl_vertex_identifier_ptr = cp_pl_vertex_identifier.as_mut_ptr();
    let cp_manifold_size_ptr = cp_manifold_size.as_mut_ptr();

    //separatrices1_points
    let mut sp_len = 0;
    const MAX_SEPARATRICES1_POINTS: usize = 100000000;
    let mut sp_id = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_POINTS);
    let mut sp_coordx = Vec::<f32>::with_capacity(MAX_SEPARATRICES1_POINTS);
    let mut sp_coordy = Vec::<f32>::with_capacity(MAX_SEPARATRICES1_POINTS);
    let mut sp_coordz = Vec::<f32>::with_capacity(MAX_SEPARATRICES1_POINTS);
    let mut sp_point_type = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_POINTS);
    let mut sp_cellid = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_POINTS);

    let sp_id_ptr = sp_id.as_mut_ptr();
    let sp_coordx_ptr = sp_coordx.as_mut_ptr();
    let sp_coordy_ptr = sp_coordy.as_mut_ptr();
    let sp_coordz_ptr = sp_coordz.as_mut_ptr();
    let sp_point_type_ptr = sp_point_type.as_mut_ptr();
    let sp_cellid_ptr = sp_cellid.as_mut_ptr();

    //separatrices1_cells
    let mut sc_len = 0;
    const MAX_SEPARATRICES1_CELLS: usize = 100000000;
    let mut sc_id = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_source = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_dest = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_connectivity_s = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_connectivity_d = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_separatrix_id = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_separatrix_type = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_f_maxima = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_f_minima = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_f_diff = Vec::<f32>::with_capacity(MAX_SEPARATRICES1_CELLS);

    let sc_id_ptr = sc_id.as_mut_ptr();
    let sc_source_ptr = sc_source.as_mut_ptr();
    let sc_dest_ptr = sc_dest.as_mut_ptr();
    let sc_connectivity_s_ptr = sc_connectivity_s.as_mut_ptr();
    let sc_connectivity_d_ptr = sc_connectivity_d.as_mut_ptr();
    let sc_separatrix_id_ptr = sc_separatrix_id.as_mut_ptr();
    let sc_separatrix_type_ptr = sc_separatrix_type.as_mut_ptr();
    let sc_f_maxima_ptr = sc_f_maxima.as_mut_ptr();
    let sc_f_minima_ptr = sc_f_minima.as_mut_ptr();
    let sc_f_diff_ptr = sc_f_diff.as_mut_ptr();

    unsafe {
        let mut my_ttk = Ttk_rs::new();
        my_ttk.simplification_3d(
            data_ptr,
            data.len() as u32,
            dim[1] as u32,
            dim[2] as u32,
            dim[0] as u32,
            authorized_birth_ptr,
            authorized_death_ptr,
            pairs.len() as u32,
            cp_point_types_ptr,
            cp_coordx_ptr,
            cp_coordy_ptr,
            cp_coordz_ptr,
            cp_value_ptr,
            cp_cellid_ptr,
            cp_pl_vertex_identifier_ptr,
            cp_manifold_size_ptr,
            &mut cp_len,
            sp_id_ptr,
            sp_coordx_ptr,
            sp_coordy_ptr,
            sp_coordz_ptr,
            sp_point_type_ptr,
            sp_cellid_ptr,
            &mut sp_len,
            sc_id_ptr,
            sc_source_ptr,
            sc_dest_ptr,
            sc_connectivity_s_ptr,
            sc_connectivity_d_ptr,
            sc_separatrix_id_ptr,
            sc_separatrix_type_ptr,
            sc_f_maxima_ptr,
            sc_f_minima_ptr,
            sc_f_diff_ptr,
            &mut sc_len,
        );
        println!("After Simplification Rust part");
        println!(
            "cp_len: {}, MAX_CRITICAL_POINTS: {}",
            cp_len, MAX_CRITICAL_POINTS
        );
        if cp_len >= MAX_CRITICAL_POINTS as u32 {
            println!("maybe segfault!");
        }
        let cp_point_types = slice::from_raw_parts(cp_point_types_ptr, cp_len as usize).to_vec();
        let cp_coordx = slice::from_raw_parts(cp_coordx_ptr, cp_len as usize).to_vec();
        let cp_coordy = slice::from_raw_parts(cp_coordy_ptr, cp_len as usize).to_vec();
        let cp_coordz = slice::from_raw_parts(cp_coordz_ptr, cp_len as usize).to_vec();
        let cp_value = slice::from_raw_parts(cp_value_ptr, cp_len as usize).to_vec();
        let cp_cellid = slice::from_raw_parts(cp_cellid_ptr, cp_len as usize).to_vec();
        let cp_pl_vertex_identifier =
            slice::from_raw_parts(cp_pl_vertex_identifier_ptr, cp_len as usize).to_vec();
        let cp_manifold_size =
            slice::from_raw_parts(cp_manifold_size_ptr, cp_len as usize).to_vec();

        println!(
            "sp_len: {}, MAX_SEPARATRICES1_POINTS: {}",
            sp_len, MAX_SEPARATRICES1_POINTS
        );
        if sp_len >= MAX_SEPARATRICES1_POINTS as u32 {
            println!("maybe segfault!");
        }

        let sp_id = slice::from_raw_parts(sp_id_ptr, sp_len as usize).to_vec();
        let sp_coordx = slice::from_raw_parts(sp_coordx_ptr, sp_len as usize).to_vec();
        let sp_coordy = slice::from_raw_parts(sp_coordy_ptr, sp_len as usize).to_vec();
        let sp_coordz = slice::from_raw_parts(sp_coordz_ptr, sp_len as usize).to_vec();
        let sp_point_type = slice::from_raw_parts(sp_point_type_ptr, sp_len as usize).to_vec();
        let sp_cellid = slice::from_raw_parts(sp_cellid_ptr, sp_len as usize).to_vec();

        println!(
            "sc_len: {}, MAX_SEPARATRICES1_CELLS: {}",
            sc_len, MAX_SEPARATRICES1_CELLS
        );
        if sc_len >= MAX_SEPARATRICES1_CELLS as u32 {
            println!("maybe segfault!");
        }

        let sc_id = slice::from_raw_parts(sc_id_ptr, sc_len as usize).to_vec();
        let sc_source = slice::from_raw_parts(sc_source_ptr, sc_len as usize).to_vec();
        let sc_dest = slice::from_raw_parts(sc_dest_ptr, sc_len as usize).to_vec();
        let sc_connectivity_s =
            slice::from_raw_parts(sc_connectivity_s_ptr, sc_len as usize).to_vec();
        let sc_connectivity_d =
            slice::from_raw_parts(sc_connectivity_d_ptr, sc_len as usize).to_vec();
        let sc_separatrix_id =
            slice::from_raw_parts(sc_separatrix_id_ptr, sc_len as usize).to_vec();
        let sc_separatrix_type =
            slice::from_raw_parts(sc_separatrix_type_ptr, sc_len as usize).to_vec();
        let sc_f_maxima = slice::from_raw_parts(sc_f_maxima_ptr, sc_len as usize).to_vec();
        let sc_f_minima = slice::from_raw_parts(sc_f_minima_ptr, sc_len as usize).to_vec();
        let sc_f_diff = slice::from_raw_parts(sc_f_diff_ptr, sc_len as usize).to_vec();
        println!("After Read from_raw_parts ///TODO");
        let mut critical_points = Vec::new();
        let mut separatrices1_points = Vec::new();
        let mut separatrices1_cells = Vec::new();
        println!("Compute CP");
        for i in 0..cp_len as usize {
            let cp = CriticalPoints::new(
                cp_point_types[i] as usize,
                (cp_coordx[i], cp_coordy[i], cp_coordz[i]),
                cp_value[i],
                cp_cellid[i] as usize,
                cp_pl_vertex_identifier[i] as usize,
                cp_manifold_size[i] as usize,
            );
            critical_points.push(cp);
        }
        println!("Compute SP");
        for i in 0..sp_len as usize {
            let sp = Separatrices1Point::new(
                sp_id[i] as usize,
                (sp_coordx[i], sp_coordy[i], sp_coordz[i]),
                sp_point_type[i] as usize,
                sp_cellid[i] as usize,
            );
            separatrices1_points.push(sp);
        }
        println!("Compute SC");
        for i in 0..sc_len as usize {
            let sc = Separatrices1Cell::new(
                sc_id[i] as usize,
                sc_source[i] as usize,
                sc_dest[i] as usize,
                (sc_connectivity_s[i] as usize, sc_connectivity_d[i] as usize),
                sc_separatrix_id[i] as usize,
                sc_separatrix_type[i] as usize,
                sc_f_maxima[i] as usize,
                sc_f_minima[i] as usize,
                sc_f_diff[i],
            );
            separatrices1_cells.push(sc);
        }
        let topology = Topology::new(critical_points, separatrices1_points, separatrices1_cells);
        out.set_topology(Some(topology));
        println!("All done!");
    }
    Ok(IOValue::Image(out))
}

fn run_compute_t_index(image: &WcsArray) -> (i32, i32, f32) {
    if let Some(topology) = image.topology() {
        //compute T-index
        let mut edgeset = Vec::new();
        let mut vset = Vec::new();
        for sc in &topology.separatrices1_cells {
            edgeset.push((sc.source, sc.dest));
            vset.push(sc.source);
            vset.push(sc.dest);
        }
        let edgeset: HashSet<(usize, usize)> = edgeset.into_iter().collect();
        let vset: HashSet<usize> = vset.into_iter().collect();
        let vset: HashSet<(usize, usize)> = vset.into_iter().enumerate().collect();
        let num_of_vertices = vset.len();
        let matrix_a = vec![0; num_of_vertices * num_of_vertices];
        let mut matrix_a = DMatrix::from_vec(num_of_vertices, num_of_vertices, matrix_a);
        let matrix_rd = vec![0.0; num_of_vertices * num_of_vertices];
        let mut matrix_rd = DMatrix::from_vec(num_of_vertices, num_of_vertices, matrix_rd);

        for (source, dest) in edgeset {
            let mut s_index = 0;
            let mut d_index = 0;
            for v in vset.clone() {
                if v.1 == source {
                    s_index = v.0;
                }
                if v.1 == dest {
                    d_index = v.0;
                }
            }
            let mut x0: f32 = 0.0;
            let mut y0: f32 = 0.0;
            let mut x1: f32 = 0.0;
            let mut y1: f32 = 0.0;
            for cp in &topology.critical_points {
                if cp.cellid == source {
                    x0 = cp.coord.0;
                    y0 = cp.coord.1;
                } else if cp.cellid == dest {
                    x1 = cp.coord.0;
                    y1 = cp.coord.1;
                }
            }
            let distance = ((x1 - x0) * (x1 - x0) + (y1 - y0) * (y1 - y0)).sqrt();
            let d = matrix_a.index_mut((s_index, d_index));
            *d = 1;
            let d = matrix_a.index_mut((d_index, s_index));
            *d = 1;

            let rd = matrix_rd.index_mut((s_index, d_index));
            *rd = distance;
            let rd = matrix_rd.index_mut((d_index, s_index));
            *rd = distance;
        }
        for i in 0..num_of_vertices {
            let d = matrix_a.index_mut((i, i));
            *d = -1;
        }
        //println!("vset: {:?}", vset);
        //println!("matrix_a: {}", matrix_a);

        let mut graph: Vec<Vec<dijkstra::Edge>> = Vec::new();
        let mut graph_distance: Vec<Vec<dijkstra::Edge>> = Vec::new();

        for i in 0..num_of_vertices {
            let mut edges = Vec::new();
            let mut edges_distance = Vec::new();
            for j in 0..num_of_vertices {
                if i != j && *matrix_a.index((i, j)) != 0 {
                    edges.push(dijkstra::Edge {
                        node: j,
                        cost: dijkstra::Total(1.0),
                    });
                    let d = matrix_rd.index((i, j));
                    edges_distance.push(dijkstra::Edge {
                        node: j,
                        cost: dijkstra::Total(*d),
                    });
                }
            }
            graph.push(edges);
            graph_distance.push(edges_distance);
        }

        let matrix_d = vec![i32::MAX; num_of_vertices * num_of_vertices];
        let mut matrix_d = DMatrix::from_vec(num_of_vertices, num_of_vertices, matrix_d);

        for i in 0..num_of_vertices {
            for j in 0..num_of_vertices {
                let d = matrix_d.index_mut((i, j));
                let rd = matrix_rd.index_mut((i, j));
                if i == j {
                    *d = -1;
                    *rd = -1.0;
                } else {
                    let dijkstra::Total(costone_distance) =
                        dijkstra::shortest_path(&graph, i, j).unwrap();
                    let dijkstra::Total(real_distance) =
                        dijkstra::shortest_path(&graph_distance, i, j).unwrap();
                    *d = costone_distance as i32;
                    *rd = 1.0 / real_distance;
                }
            }
        }
        //println!("matrix_d: {}", matrix_d);
        let i_a = compute_abs_sum_of_coefs_of_characteristic_polynomial(&matrix_a).unwrap();
        let i_d = compute_abs_sum_of_coefs_of_characteristic_polynomial(&matrix_d).unwrap();
        let f_d = compute_abs_sum_of_coefs_of_characteristic_polynomial_float(&matrix_rd).unwrap();
        println!("matrix_a: {}", matrix_a);
        println!("matrix_d: {}", matrix_d);
        println!("matrix_rd: {}", matrix_rd);
        println!("({}, {}, {})", i_a, i_d, f_d);
        (i_a, i_d, f_d)
    } else {
        (-1, -1, -1.0)
    }
}

fn compute_abs_sum_of_coefs_of_characteristic_polynomial(m: &DMatrix<i32>) -> Option<i32> {
    let shape = m.shape();
    let mut ret = None;
    if shape.0 == shape.1 {
        let m_size = shape.0;
        let mut coordsb: Vec<usize> = (0..m_size).into_iter().collect();

        let mut sums = vec![0; m_size + 1];
        use permutator::Permutation;

        let mut counter = 0;
        coordsb.permutation().for_each(|p| {
            let coords: Vec<(usize, usize)> = (0..m_size).zip(p).collect();
            let mut vals = 1;
            let mut num_of_minus_one = 0;
            let mut is_zero = false;
            for (x, y) in coords {
                if *m.index((x, y)) == -1 {
                    num_of_minus_one += 1;
                } else if *m.index((x, y)) == 0 {
                    is_zero = true;
                    break;
                }
                vals *= m.index((x, y));
            }
            if !is_zero {
                counter += 1;
                let iseven = if counter % 2 == 0 { true } else { false };
                vals *= if iseven { 1 } else { -1 };
                sums[num_of_minus_one] += vals;
            }
        });
        let sum: i32 = sums.into_iter().map(|v: i32| v.abs()).sum();
        ret = Some(sum);
    }
    ret
}

fn compute_abs_sum_of_coefs_of_characteristic_polynomial_float(m: &DMatrix<f32>) -> Option<f32> {
    let shape = m.shape();
    let mut ret = None;
    if shape.0 == shape.1 {
        let m_size = shape.0;
        let mut coordsb: Vec<usize> = (0..m_size).into_iter().collect();

        let mut sums = vec![0.0; m_size + 1];
        use permutator::Permutation;

        let mut counter = 0;
        coordsb.permutation().for_each(|p| {
            let coords: Vec<(usize, usize)> = (0..m_size).zip(p).collect();
            let mut vals = 1.0;
            let mut num_of_minus_one = 0;
            let mut is_zero = false;
            for (x, y) in coords {
                if *m.index((x, y)) == -1.0 {
                    num_of_minus_one += 1;
                } else if *m.index((x, y)) == 0.0 {
                    is_zero = true;
                    break;
                }
                vals *= m.index((x, y));
            }
            if !is_zero {
                counter += 1;
                let iseven = if counter % 2 == 0 { true } else { false };
                vals *= if iseven { 1.0 } else { -1.0 };
                sums[num_of_minus_one] += vals;
            }
        });
        let sum: f32 = sums.into_iter().map(|v: f32| v.abs()).sum();
        ret = Some(sum);
    }
    ret
}

fn run_ttk_ftr(image: &WcsArray) -> Result<IOValue, IOErr> {
    dim_is!(image, 3)?;
    let mut anormal_idx = Vec::new();
    let mut result = Vec::new();
    for (k, slice) in image.scalar().axis_iter(Axis(0)).enumerate() {
        println!("idx:{}", k);
        let s = slice.into_owned();
        let image_2d = WcsArray::from_array(Dimensioned::new(s.into_dyn(), Unit::None));
        if let IOValue::PersistencePairs(pp) = run_ttk_persistence_pairs(&image_2d)? {
            if let IOValue::PersistencePairs(pp) = run_select_the_most_pairs_using_sigma(pp, 0.3)? {
                let PersistencePairs::Pairs(_) = pp.clone();
                if let IOValue::Image(image) = run_ttk_simplification(&image_2d, pp)? {
                    if let Some(topology) = image.topology() {
                        if topology.critical_points.len() > 12 {
                            println!("first time: failured {}", topology.critical_points.len());
                            anormal_idx.push(k);
                            result.push((-1, -1, -1.0));
                        } else {
                            result.push(run_compute_t_index(&image));
                        }
                    }
                    //use std::{thread, time};
                    //thread::sleep(time::Duration::from_millis(500));
                    //run_compute_t_index(&image);
                }
            }
        }
    }
    for idx in anormal_idx {
        println!("idx: {}", idx);
        let slice = image.scalar().index_axis(Axis(0), idx);
        let s = slice.into_owned();
        let image_2d = WcsArray::from_array(Dimensioned::new(s.into_dyn(), Unit::None));
        if let IOValue::PersistencePairs(pp) = run_ttk_persistence_pairs(&image_2d)? {
            if let IOValue::PersistencePairs(pp) = run_select_the_most_pairs(pp, 0.1)? {
                let PersistencePairs::Pairs(_) = pp.clone();
                if let IOValue::Image(image) = run_ttk_simplification(&image_2d, pp)? {
                    if let Some(topology) = image.topology() {
                        if topology.critical_points.len() > 12 {
                            println!("second time: failured {}", topology.critical_points.len());
                            result[idx] = (-1, -1, -1.0);
                        } else {
                            result[idx] = run_compute_t_index(&image);
                        }
                    }
                    //use std::{thread, time};
                    //thread::sleep(time::Duration::from_millis(500));
                    //run_compute_t_index(&image);
                }
            }
        }
    }
    println!("i_a:");
    for t in &result {
        println!("{}", t.0);
    }
    println!("i_d:");
    for t in &result {
        println!("{}", t.1);
    }
    Ok(IOValue::Image(image.clone()))
}

fn run_ttk_create_kappa_lambda_map(image: &WcsArray) -> Result<IOValue, IOErr> {
    dim_is!(image, 3)?;
    let mut result = Vec::new();
    let image_val = image.scalar();
    let wavesize = *image_val.dim().as_array_view().first().unwrap();
    let n = 200;
    for i in 0..n {
        for (k, slice) in image.scalar().axis_iter(Axis(0)).enumerate() {
            println!("now_kappa: {}, wave: {}", i as f32 * 0.01, k);
            let s = slice.into_owned();
            let image_2d = WcsArray::from_array(Dimensioned::new(s.into_dyn(), Unit::None));
            if let IOValue::PersistencePairs(pp) = run_ttk_persistence_pairs(&image_2d)? {
                if let IOValue::PersistencePairs(PersistencePairs::Pairs(data)) =
                    run_select_the_most_pairs_using_sigma(pp, i as f32 * 0.01)?
                {
                    result.push(data.len() as f32);
                }
            }
        }
    }
    let img = Array::from_shape_vec((n, wavesize), result).unwrap();
    Ok(IOValue::Image(WcsArray::from_array(Dimensioned::new(
        img.into_dyn(),
        Unit::None,
    ))))
}

fn run_ttk_create_tindexes_map(image: &WcsArray) -> Result<IOValue, IOErr> {
    dim_is!(image, 3)?;
    let mut result_i = Vec::new();
    let mut result_d = Vec::new();
    let mut result_pp = Vec::new();
    let image_val = image.scalar();
    let wavesize = *image_val.dim().as_array_view().first().unwrap();
    let n = 100;
    for i in 0..n {
        for (k, slice) in image.scalar().axis_iter(Axis(0)).enumerate() {
            println!("now_kappa: {}, wave: {}", i as f32 * 0.01, k);
            let s = slice.into_owned();
            let image_2d = WcsArray::from_array(Dimensioned::new(s.into_dyn(), Unit::None));
            if let IOValue::PersistencePairs(pp) = run_ttk_persistence_pairs(&image_2d)? {
                if let IOValue::PersistencePairs(pp) =
                    run_select_the_most_pairs_using_sigma(pp, i as f32 * 0.01)?
                {
                    let PersistencePairs::Pairs(data) = pp.clone();
                    if data.len() == 0 {
                        println!("Empty Pairs!");
                    }
                    if let IOValue::Image(image) = run_ttk_simplification(&image_2d, pp)? {
                        if let Some(topology) = image.topology() {
                            if topology.critical_points.len() > 10 {
                                result_i.push(-1.0);
                                result_d.push(-1.0);
                                result_pp.push(0.0);
                            } else {
                                let t_idxes = run_compute_t_index(&image);
                                result_i.push(t_idxes.0 as f32);
                                result_d.push(t_idxes.1 as f32);
                                result_pp.push(t_idxes.2);
                            }
                        }
                    }
                }
            }
        }
    }
    let p_max = result_pp.iter().fold(0.0 / 0.0, |m, v| v.max(m));
    let p_min = result_pp.iter().fold(-0.0 / 0.0, |m, v| v.min(m));
    let mut result_pp = result_pp
        .iter()
        .map(|v| (*v - p_min) / (p_max - p_min) * 100.0)
        .collect::<Vec<f32>>();

    let i_max = result_i.iter().fold(0.0 / 0.0, |m, v| v.max(m));
    let d_max = result_d.iter().fold(0.0 / 0.0, |m, v| v.max(m));
    let result_i = result_i
        .iter()
        .map(|v| if *v == -1.0 { i_max } else { *v })
        .collect::<Vec<f32>>();
    let result_d = result_d
        .iter()
        .map(|v| if *v == -1.0 { d_max } else { *v })
        .collect::<Vec<f32>>();
    let i_min = result_i.iter().fold(-0.0 / 0.0, |m, v| v.min(m));
    let d_min = result_d.iter().fold(-0.0 / 0.0, |m, v| v.min(m));
    println!(
        "(i_max, i_min, d_max, d_min, p_max, p_min) = ({}, {}, {}, {}, {}, {})",
        i_max, i_min, d_max, d_min, p_max, p_min
    );
    let mut result_i = result_i
        .iter()
        .map(|v| (*v - i_min) / (i_max - i_min) * 255.0 - 128.0)
        .collect::<Vec<f32>>();
    let mut result_d = result_d
        .iter()
        .map(|v| (*v - d_min) / (d_max - d_min) * 255.0 - 128.0)
        .collect::<Vec<f32>>();
    result_pp.append(&mut result_i);
    result_pp.append(&mut result_d);

    let img = Array::from_shape_vec((3, n, wavesize), result_pp).unwrap();
    Ok(IOValue::Image(WcsArray::from_array(Dimensioned::new(
        img.into_dyn(),
        Unit::None,
    ))))
}

impl cake::NamedAlgorithms<IOErr> for IOValue {
    fn get_transform(s: &str) -> Option<&'static cake::Transform<'static, IOValue, IOErr>> {
        for t in TRANSFORMATIONS.iter() {
            if t.name() == s {
                return Some(t);
            }
        }
        None
    }
}

fn func(x: f32, y: f32, z: f32, c: f32, d: f32, r: f32) -> f32 {
    4.0 * c * c * ((x - r) * (x - r) + (z - r) * (z - r) + (x + r) * (x + r) + (z + r) * (z + r))
        - ((x - r) * (x - r) + y * y + (z - r) * (z - r) + c * c - d * d)
            * ((x - r) * (x - r) + y * y + (z - r) * (z - r) + c * c - d * d)
        - ((x + r) * (x + r) + y * y + (z + r) * (z + r) + c * c - d * d)
            * ((x + r) * (x + r) + y * y + (z + r) * (z + r) + c * c - d * d)
}

impl cake::DefaultFor for IOValue {
    fn default_for(variant_name: &str) -> Self {
        match variant_name {
            "Integer" => IOValue::Integer(0),
            "Float" => IOValue::Float(0.0),
            "Float2" => IOValue::Float2([0.0; 2]),
            "ToneCurve" => IOValue::ToneCurve(ToneCurveState::default()),
            "Float3" => IOValue::Float3([0.0; 3]),
            "Float3x3" => IOValue::Float3x3([[0.0; 3]; 3]),
            "Roi" => IOValue::Roi(roi::ROI::All),
            "Str" => IOValue::Str("".to_owned()),
            "Bool" => IOValue::Bool(false),
            "Paths" => IOValue::Paths(PATHS::FileList(vec![])),
            "Sliced_Analytic_Volume" => {
                let image_data = {
                    const WIDTH: usize = 129;
                    const HEIGHT: usize = 129;
                    const C: f32 = 0.6;
                    const D: f32 = 0.5;
                    const R: f32 = 0.2;
                    let mut image_data = Vec::with_capacity(WIDTH * HEIGHT);
                    for j in 0..WIDTH {
                        for i in 0..HEIGHT {
                            let j = (j as f32) / 64.0 - 1.0;
                            let i = (i as f32) / 64.0 - 1.0;
                            let v = func(j as f32, 0.0, i as f32, C, D, R);
                            image_data.push(v);
                        }
                    }
                    ndarray::ArrayD::from_shape_vec(vec![WIDTH, HEIGHT], image_data).unwrap()
                };
                IOValue::Image(WcsArray::from_array_and_tag(
                    Dimensioned::new(image_data.into_dyn(), Unit::None),
                    None,
                ))
            }
            "Analytic_Volume" => {
                let image_data = {
                    const WIDTH: usize = 128;
                    const HEIGHT: usize = 128;
                    const DEPTH: usize = 128;
                    const C: f32 = 0.6;
                    const D: f32 = 0.5;
                    const R: f32 = 0.2;
                    let mut image_data = Vec::with_capacity(WIDTH * HEIGHT * DEPTH);
                    for j in 0..WIDTH {
                        for i in 0..HEIGHT {
                            for k in 0..DEPTH {
                                let j = (j as f32) / 64.0 - 1.0;
                                let i = (i as f32) / 64.0 - 1.0;
                                let k = (k as f32) / 64.0 - 1.0;
                                let v = func(j as f32, k, i as f32, C, D, R);
                                image_data.push(v);
                            }
                        }
                    }
                    ndarray::ArrayD::from_shape_vec(vec![WIDTH, HEIGHT, DEPTH], image_data).unwrap()
                };
                IOValue::Image(WcsArray::from_array_and_tag(
                    Dimensioned::new(image_data.into_dyn(), Unit::None),
                    None,
                ))
            }
            _ => panic!("Unknown variant name provided: {}.", variant_name),
        }
    }
}

impl cake::EditableVariants for IOValue {
    fn editable_variants() -> &'static [&'static str] {
        &[
            "Integer",
            "Float",
            "Float2",
            "Float3",
            "Float3x3",
            "Roi",
            "Str",
            "Bool",
            "Paths",
            "ToneCurve",
            "Sliced_Analytic_Volume",
            "Analytic_Volume",
        ]
    }
}

impl cake::ConvertibleVariants for IOValue {
    const CONVERTION_TABLE: &'static [cake::ConvertibleVariant<Self>] = &[
        cake::ConvertibleVariant {
            from: "Integer",
            into: "Float",
            f: integer_to_float,
        },
        cake::ConvertibleVariant {
            from: "Float",
            into: "Integer",
            f: float_to_integer,
        },
    ];
}

fn integer_to_float(from: &IOValue) -> IOValue {
    if let IOValue::Integer(int) = from {
        IOValue::Float(*int as f32)
    } else {
        panic!("Unexpected input!")
    }
}
fn float_to_integer(from: &IOValue) -> IOValue {
    if let IOValue::Float(f) = from {
        IOValue::Integer(f.round() as _)
    } else {
        panic!("Unexpected input!")
    }
}

/// Open FITS file
fn run_open_fits<P: AsRef<Path>>(path: Vec<P>, n: i64) -> Result<IOValue, IOErr> {
    let pathlist_len = path.len();
    let n = n as usize;
    precheck!(pathlist_len > n)?;
    let path = &path[n];
    let path = path.as_ref();
    fitrs::Fits::open(path)
        .map(|fits| IOValue::Fits(Arc::new(fits)))
        .map_err(|err| IOErr::IoError(err, format!("Could not open file {:?}", path)))
}

fn run_open_raw<P: AsRef<Path>>(path: Vec<P>, n: i64) -> Result<IOValue, IOErr> {
    let pathlist_len = path.len();
    let n = n as usize;
    precheck!(pathlist_len > n)?;
    let mut imagecount = 0;
    let mut width = 0;
    let mut height = 0;
    let mut img = Vec::new();
    let mut openresult: Result<IOValue, IOErr> = Ok(IOValue::Integer(0));
    for single_path in path {
        let image = rawloader::decode_file(single_path);
        match image {
            Ok(image) => {
                if imagecount == 0 {
                    width = image.width;
                    height = image.height;
                } else if width != image.width || height != image.height {
                    eprintln!("Couldn't load images with different size images.\n");
                    break;
                }
                if let rawloader::RawImageData::Integer(data) = image.data {
                    for pix in data {
                        img.push(pix as f32);
                    }
                } else {
                    eprintln!("Don't know how to process non-integer raw files");
                    break;
                }
            }
            Err(err) => openresult = Err(IOErr::RawLoaderError(format!("{:?}", err))),
        }
        imagecount += 1;
    }
    match openresult {
        Ok(_) => {
            let img = Array::from_shape_vec((pathlist_len, height, width), img).unwrap();
            Ok(IOValue::Image(WcsArray::from_array(Dimensioned::new(
                img.into_dyn(),
                Unit::None,
            ))))
        }
        Err(_) => openresult,
    }
}

/// Turn a FITS file into an image
fn run_fits_to_image(
    fits: &Arc<fitrs::Fits>,
    hdu_idx: i64,
    extension: &str,
) -> Result<IOValue, IOErr> {
    let hdu_idx = try_into_unsigned!(hdu_idx)?;
    let primary_hdu = fits
        .get_by_name(extension)
        .or_else(|| fits.get(hdu_idx))
        .ok_or_else(|| {
            let hdu_name = if hdu_idx == 0 {
                "Primary HDU".to_owned()
            } else {
                format!("HDU #{}", hdu_idx)
            };
            if extension == "" {
                IOErr::UnexpectedInput(format!("Could not find {} in FITS file.", hdu_name))
            } else {
                IOErr::UnexpectedInput(format!(
                    "Could not find HDU '{}', nor {} in FITS file.",
                    extension, hdu_name
                ))
            }
        })?;
    WcsArray::from_hdu(&primary_hdu)
        .map(IOValue::Image)
        .map_err(|e| IOErr::FITSErr(format!("{}", e)))
}

fn run_slice_one_frame(input_img: &WcsArray, frame_idx: i64) -> Result<IOValue, IOErr> {
    let frame_idx = try_into_unsigned!(frame_idx)?;
    is_sliceable!(input_img, frame_idx)?;

    let image_val = input_img.scalar();
    let out = image_val.index_axis(Axis(0), frame_idx);

    let wrap_with_unit = input_img.make_slice(
        &(0..out.ndim()).map(|i| (i, 0.0, 1.0)).collect::<Vec<_>>(),
        input_img.array().with_new_value(out.to_owned()),
    );

    Ok(IOValue::Image(wrap_with_unit))
}

/// Slice a 3D image through an arbitrary 2D plane
fn run_slice_3d_to_2d(image: &WcsArray, map: &Array2<[f32; 3]>) -> Result<IOValue, IOErr> {
    dim_is!(image, 3)?;

    use std::f32::EPSILON;

    #[derive(Debug)]
    struct MapReverseParams<'a> {
        origin: Option<&'a [f32; 3]>,
        dir1: [Option<f32>; 3],
        dir2: [Option<f32>; 3],
    }
    enum DirValue {
        Unset,
        None,
        Some(f32),
    }
    impl DirValue {
        fn into_option(self) -> Option<f32> {
            match self {
                DirValue::Unset | DirValue::None => None,
                DirValue::Some(f) => Some(f),
            }
        }

        fn update(&mut self, test_d: f32) {
            match *self {
                DirValue::Unset => *self = DirValue::Some(test_d),
                DirValue::Some(dx_) => {
                    if (dx_ - test_d).abs() > EPSILON {
                        *self = DirValue::None;
                    }
                }
                DirValue::None => (),
            }
        }
    }
    impl<'a> MapReverseParams<'a> {
        fn new(map: &'a Array2<[f32; 3]>) -> Self {
            let (n, m) = map.dim();

            let dir1 = {
                let mut dx = DirValue::Unset;
                let mut dy = DirValue::Unset;
                let mut dz = DirValue::Unset;
                for i in 0..n {
                    for j in 0..(m - 1) {
                        let [x0, y0, z0] = map[(i, j)];
                        let [x1, y1, z1] = map[(i, j + 1)];
                        {
                            let test_dx = x1 - x0;
                            dx.update(test_dx);
                        }
                        {
                            let test_dy = y1 - y0;
                            dy.update(test_dy);
                        }
                        {
                            let test_dz = z1 - z0;
                            dz.update(test_dz);
                        }
                    }
                }
                [dx.into_option(), dy.into_option(), dz.into_option()]
            };
            let dir2 = {
                let mut dx = DirValue::Unset;
                let mut dy = DirValue::Unset;
                let mut dz = DirValue::Unset;
                for i in 0..(n - 1) {
                    for j in 0..m {
                        let [x0, y0, z0] = map[(i, j)];
                        let [x1, y1, z1] = map[(i + 1, j)];
                        {
                            let test_dx = x1 - x0;
                            dx.update(test_dx);
                        }
                        {
                            let test_dy = y1 - y0;
                            dy.update(test_dy);
                        }
                        {
                            let test_dz = z1 - z0;
                            dz.update(test_dz);
                        }
                    }
                }
                [dx.into_option(), dy.into_option(), dz.into_option()]
            };

            let origin = map.get((0, 0));
            Self { origin, dir1, dir2 }
        }

        fn sliced_axes(&self) -> Option<[(usize, f32, f32); 2]> {
            enum HowToSlice {
                KeepDirection,
                RemoveDirection,
            }
            impl HowToSlice {
                fn how(dx1: Option<f32>, dx2: Option<f32>) -> Option<Self> {
                    if let (Some(dx1), Some(dx2)) = (dx1, dx2) {
                        if dx1 < EPSILON && dx2 > EPSILON || dx1 > EPSILON && dx2 < EPSILON {
                            Some(HowToSlice::KeepDirection)
                        } else if dx1 < EPSILON && dx2 < EPSILON {
                            Some(HowToSlice::RemoveDirection)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
            }

            let remove_x = match HowToSlice::how(self.dir1[0], self.dir2[0]) {
                Some(HowToSlice::KeepDirection) => false,
                Some(HowToSlice::RemoveDirection) => true,
                None => false,
            };
            let remove_y = match HowToSlice::how(self.dir1[1], self.dir2[1]) {
                Some(HowToSlice::KeepDirection) => false,
                Some(HowToSlice::RemoveDirection) => true,
                None => false,
            };
            let remove_z = match HowToSlice::how(self.dir1[2], self.dir2[2]) {
                Some(HowToSlice::KeepDirection) => false,
                Some(HowToSlice::RemoveDirection) => true,
                None => false,
            };

            if let Some(origin) = self.origin {
                match (remove_x, remove_y, remove_z) {
                    (true, false, false) => self.non_zero_factor(2).and_then(|factor2| {
                        self.non_zero_factor(1)
                            .map(|factor1| [(0, origin[2], factor2), (1, origin[1], factor1)])
                    }),
                    (false, true, false) => self.non_zero_factor(2).and_then(|factor2| {
                        self.non_zero_factor(0)
                            .map(|factor0| [(0, origin[2], factor2), (2, origin[0], factor0)])
                    }),
                    (false, false, true) => self.non_zero_factor(1).and_then(|factor1| {
                        self.non_zero_factor(0)
                            .map(|factor0| [(1, origin[1], factor1), (2, origin[0], factor0)])
                    }),
                    _ => None,
                }
            } else {
                None
            }
        }
        fn non_zero_factor(&self, index: usize) -> Option<f32> {
            match (self.dir1[index], self.dir2[index]) {
                (Some(x), Some(y)) => {
                    if x.abs() < EPSILON {
                        Some(y)
                    } else {
                        Some(x)
                    }
                }
                _ => None,
            }
        }
    }

    let mut out = Vec::with_capacity(map.len());
    for &[x, y, z] in map {
        // Interpolate to nearest
        let input_img = image.scalar();

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
        .map(|array| {
            let array = array.into_dyn();
            let array = image.array().with_new_value(array);
            let params = MapReverseParams::new(map);
            let array = if let Some(axes) = params.sliced_axes() {
                image.make_slice(&axes, array)
            } else {
                WcsArray::from_array(array)
            };
            IOValue::Image(array)
        })
        .map_err(|e| IOErr::ShapeError(e, "slice3d_to_2d: Unexpected error".to_owned()))
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
    let count1 = try_into_unsigned!(count1)?;
    let count2 = try_into_unsigned!(count2)?;

    let map = Array2::from_shape_fn((count2, count1), |(j, i)| {
        let i = i as f32;
        let j = j as f32;
        [
            x0 + i * dx1 + j * dx2,
            y0 + i * dy1 + j * dy2,
            z0 + i * dz1 + j * dz2,
        ]
    });
    Ok(IOValue::Map2dTo3dCoords(map))
}

fn run_extract_wave(image: &WcsArray, roi: &roi::ROI) -> Result<IOValue, IOErr> {
    dim_is!(image, 3)?;

    let image_val = image.scalar();

    let wave_size = *image_val.dim().as_array_view().first().unwrap();
    let mut wave = Vec::with_capacity(wave_size);
    for i in 0..wave_size {
        let mut res = 0.0;
        for (_, val) in roi.filter(image_val.slice(s![i, .., ..])) {
            if !val.is_nan() {
                res += val;
            }
        }
        wave.push(res);
    }
    Ok(IOValue::Image(
        image.make_slice(
            &[(2, 0.0, 1.0)],
            image
                .array()
                .with_new_value(Array1::from_vec(wave).into_dyn()),
        ),
    ))
}

fn run_create_scatter(xaxis: &WcsArray, yaxis: &WcsArray) -> Result<IOValue, IOErr> {
    are_same_dim!(xaxis, yaxis)?;
    let x_axis = xaxis.scalar();
    let y_axis = yaxis.scalar();
    let mut imgx = Vec::new();
    let mut imgy = Vec::new();
    let mut indexes = Vec::new();
    let mut index_size = 0;
    for (idx, d) in x_axis.indexed_iter() {
        imgx.push(*d);
        index_size = idx.ndim();
        indexes.push(idx);
    }
    for (_, d) in y_axis.indexed_iter() {
        imgy.push(*d);
    }
    let mut datapoints = Vec::new();
    for i in 0..imgx.len() {
        datapoints.push((imgx[i], imgy[i], 0.0, indexes[i].clone()));
    }

    datapoints.retain(|x| (x.0.is_finite() && x.1.is_finite()));
    let mut center = (0.0, 0.0);
    let datalen = datapoints.len();
    for i in 0..datalen {
        let vector = (datapoints[i].0 - center.0, datapoints[i].1 - center.1);
        center.0 += vector.0 / (i + 1) as f32;
        center.1 += vector.1 / (i + 1) as f32;
    }
    for i in 0..datalen {
        let distance = ((datapoints[i].0 - center.0) * (datapoints[i].0 - center.0)
            + (datapoints[i].1 - center.1) * (datapoints[i].1 - center.1))
            .sqrt();
        datapoints[i].2 = distance;
    }
    datapoints.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());
    let mut res = Vec::with_capacity(3 * datalen);
    for i in 0..datalen {
        res.push(datapoints[i].0);
    }
    for i in 0..datalen {
        res.push(datapoints[i].1);
    }
    for i in 0..datalen {
        res.push(datapoints[i].2);
    }
    for j in 0..index_size {
        for i in 0..datalen {
            res.push(datapoints[i].3[j] as f32);
        }
    }
    let img = Array::from_shape_vec((3 + index_size, datalen), res).unwrap();
    Ok(IOValue::Image(WcsArray::from_array_and_tag(
        Dimensioned::new(img.into_dyn(), Unit::None),
        Some(String::from("scatter")),
    )))
}

fn run_range_specification(image: &WcsArray, start: i64, end: i64) -> Result<IOValue, IOErr> {
    reduce_array_slice(image, start, end, |slices| slices.to_owned())
}

fn run_range_specification_x(image: &WcsArray, start: i64, end: i64) -> Result<IOValue, IOErr> {
    reduce_array_slice_x(image, start, end, |slices| slices.to_owned())
}

fn run_range_specification_y(image: &WcsArray, start: i64, end: i64) -> Result<IOValue, IOErr> {
    reduce_array_slice_y(image, start, end, |slices| slices.to_owned())
}

fn run_extrude(image: &WcsArray, roi: &roi::ROI) -> Result<IOValue, IOErr> {
    dim_is!(image, 3)?;

    let image_val = image.scalar();
    let wave_size = *image_val.dim().as_array_view().first().unwrap();

    let new_size = (wave_size, roi.datalen());
    let mut result = Vec::with_capacity(wave_size * roi.datalen());

    for i in (0..wave_size).rev() {
        for (_, val) in roi.filter(image_val.slice(s![i, .., ..])) {
            result.push(val);
        }
    }

    let waveimg = Array::from_shape_vec(new_size.strides((roi.datalen(), 1)), result).unwrap();
    let unit = image.array().unit();

    // FIXME: handle metadata
    Ok(IOValue::Image(WcsArray::from_array(Dimensioned::new(
        waveimg.into_dyn(),
        unit.to_owned(),
    ))))
}

fn run_clip(
    image: &WcsArray,
    ceiling_threshold: f32,
    ceiling: bool,
    floor_threshold: f32,
    floor: bool,
) -> Result<IOValue, IOErr> {
    let mut image = image.clone();

    for f in image.scalar_mut().iter_mut() {
        if (ceiling && *f >= ceiling_threshold) || (floor && *f <= floor_threshold) {
            *f = ::std::f32::NAN;
        }
    }

    Ok(IOValue::Image(image))
}

fn run_clip_background(
    image: &WcsArray,
    sigma_high: f32,
    sigma_low: f32,
    alpha: f32,
) -> Result<IOValue, IOErr> {
    let original_meta = image.meta();
    let original_visualization = image.tag();
    let original_topology = image.topology();
    let mut image = image.clone();
    let image_val = image.scalar_mut();

    let mut backgrounds = Vec::new();
    for slice in image_val.axis_iter(Axis(0)) {
        let d = Dimensioned::new(slice.to_owned(), Unit::None);
        let image = WcsArray::new(
            original_meta.to_owned(), //FIXME: Handle metadata
            d,
            original_visualization.to_owned(),
            original_topology.to_owned(),
        );
        let (_, _, _, background) = run_compute_background(&image, sigma_high, sigma_low, alpha);
        backgrounds.push(background);
    }
    for (slice, background) in image_val.axis_iter_mut(Axis(0)).zip(backgrounds) {
        for f in slice {
            if *f <= background {
                *f = ::std::f32::NAN;
            }
        }
    }
    Ok(IOValue::Image(WcsArray::from_array(Dimensioned::new(
        image_val.clone(),
        Unit::None,
    ))))
}

fn run_replace_nan_image(image: &WcsArray, placeholder: f32) -> Result<IOValue, IOErr> {
    let mut image = image.clone();

    for f in image.scalar_mut().iter_mut() {
        if f.is_nan() {
            *f = placeholder;
        }
    }

    Ok(IOValue::Image(image))
}

fn run_linear_composition(
    i1: &WcsArray,
    i2: &WcsArray,
    coef1: f32,
    coef2: f32,
) -> Result<IOValue, IOErr> {
    are_same_dim!(i1, i2)?;
    let out = i1 * coef1 + i2 * coef2;
    Ok(IOValue::Image(out))
}

fn run_make_float3(f1: f32, f2: f32, f3: f32) -> Result<IOValue, IOErr> {
    Ok(IOValue::Float3([f1, f2, f3]))
}

fn run_compute_background(
    image: &WcsArray,
    sigma_high: f32,
    sigma_low: f32,
    alpha: f32,
) -> (f32, f32, f32, f32) {
    let image_arr = image.scalar();
    let mut data_arr = Vec::new();
    let mut data_sum = 0.0;
    for data in image_arr {
        if !data.is_nan() {
            data_arr.push(*data);
            data_sum += *data;
        }
    }
    let mut data_len = data_arr.len();
    data_arr.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut mean = data_sum / data_len as f32;
    let mut median = if data_len % 2 == 0 {
        (data_arr[data_len / 2 - 1] + data_arr[data_len / 2]) / 2.0
    } else {
        data_arr[data_len / 2]
    };
    let mut variance = 0.0;
    for i in &data_arr {
        variance += (i - mean) * (i - mean);
    }
    variance /= data_len as f32;
    let mut stddev = variance.sqrt();
    loop {
        data_arr.retain(|&d| mean - sigma_low * stddev < d && d < mean + sigma_high * stddev);
        let new_data_len = data_arr.len();
        if data_len != new_data_len {
            let mut new_sum = 0.0;
            for d in &data_arr {
                new_sum += d;
            }
            mean = new_sum / new_data_len as f32;
            median = if new_data_len % 2 == 0 {
                (data_arr[new_data_len / 2 - 1] + data_arr[new_data_len / 2]) / 2.0
            } else {
                data_arr[new_data_len / 2]
            };
            variance = 0.0;
            for i in &data_arr {
                variance += (i - mean) * (i - mean);
            }
            variance /= new_data_len as f32;
            stddev = variance.sqrt();
            data_len = new_data_len;
        } else {
            break;
        }
    }
    let background = median + alpha * stddev;
    (mean, median, stddev, background)
}

fn reduce_array_slice<F>(image: &WcsArray, start: i64, end: i64, f: F) -> Result<IOValue, IOErr>
where
    F: Fn(&ArrayViewD<f32>) -> ArrayD<f32>,
{
    let start = try_into_unsigned!(start)?;
    let end = try_into_unsigned!(end)?;
    is_sliceable!(image, start, end)?;

    let image_val = image.scalar();

    let slices = image_val.slice_axis(Axis(0), Slice::from(start..end));
    let raw = f(&slices);
    let ndim = raw.ndim();

    let wrap_with_unit = image.make_slice(
        &(0..ndim).map(|i| (i, 0.0, 1.0)).collect::<Vec<_>>(),
        image.array().with_new_value(raw),
    );
    Ok(IOValue::Image(wrap_with_unit))
}

fn reduce_array_slice_x<F>(image: &WcsArray, start: i64, end: i64, f: F) -> Result<IOValue, IOErr>
where
    F: Fn(&ArrayViewD<f32>) -> ArrayD<f32>,
{
    dim_is!(image, 3)?;
    let xdim = image.scalar().dim().as_array_view()[1];
    let start = try_into_unsigned!(start)?;
    let end = try_into_unsigned!(end)?;
    if xdim < start || xdim < end || end < start {
        return Err(IOErr::UnexpectedInput(format!(
            "Cannot slice {}, {}, {}",
            start, end, xdim
        )));
    }

    let image_val = image.scalar();

    let slices = image_val.slice_axis(Axis(1), Slice::from(start..end));
    let raw = f(&slices);
    let ndim = raw.ndim();

    let wrap_with_unit = image.make_slice(
        &(0..ndim).map(|i| (i, 0.0, 1.0)).collect::<Vec<_>>(),
        image.array().with_new_value(raw),
    );
    Ok(IOValue::Image(wrap_with_unit))
}

fn reduce_array_slice_y<F>(image: &WcsArray, start: i64, end: i64, f: F) -> Result<IOValue, IOErr>
where
    F: Fn(&ArrayViewD<f32>) -> ArrayD<f32>,
{
    dim_is!(image, 3)?;
    let ydim = image.scalar().dim().as_array_view()[2];
    let start = try_into_unsigned!(start)?;
    let end = try_into_unsigned!(end)?;
    if ydim < start || ydim < end || end < start {
        return Err(IOErr::UnexpectedInput(format!(
            "Cannot slice {}, {}, {}",
            start, end, ydim
        )));
    }
    let image_val = image.scalar();

    let slices = image_val.slice_axis(Axis(2), Slice::from(start..end));
    let raw = f(&slices);
    let ndim = raw.ndim();

    let wrap_with_unit = image.make_slice(
        &(0..ndim).map(|i| (i, 0.0, 1.0)).collect::<Vec<_>>(),
        image.array().with_new_value(raw),
    );
    Ok(IOValue::Image(wrap_with_unit))
}

fn run_integral(image: &WcsArray, start: i64, end: i64) -> Result<IOValue, IOErr> {
    reduce_array_slice(image, start, end, |slices| slices.sum_axis(Axis(0)))
}

fn run_average(image: &WcsArray, start: i64, end: i64) -> Result<IOValue, IOErr> {
    reduce_array_slice(image, start, end, |slices| slices.mean_axis(Axis(0)))
}

fn run_variance(image: &WcsArray, start: i64, end: i64) -> Result<IOValue, IOErr> {
    reduce_array_slice(image, start, end, |slices| slices.var_axis(Axis(0), 1.0))
}

fn run_stddev(image: &WcsArray, start: i64, end: i64) -> Result<IOValue, IOErr> {
    reduce_array_slice(image, start, end, |slices| slices.std_axis(Axis(0), 1.0))
}

fn run_median(image: &WcsArray, start: i64, end: i64) -> Result<IOValue, IOErr> {
    let start = try_into_unsigned!(start)?;
    let end = try_into_unsigned!(end)?;
    is_sliceable!(image, start, end)?;

    let image_val = image.scalar();

    let slices = image_val.slice_axis(Axis(0), Slice::from(start..end));
    let dim = slices.dim();
    let size = dim.as_array_view();
    let new_size: Vec<_> = size.iter().skip(1).cloned().collect();

    let result = ArrayD::from_shape_fn(new_size, |index| {
        let mut vals = Vec::new();
        for (_, slice) in slices.axis_iter(Axis(0)).enumerate() {
            vals.push(slice[&index]);
        }
        //consider NaN!
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = vals.len();
        if n % 2 == 1 {
            vals[(n - 1) / 2]
        } else {
            (vals[n / 2] + vals[n / 2 - 1]) / 2.0
        }
    });

    // FIXME: Unit support
    // unit of index(Axis 0) should be adobped
    //
    // in above program...
    // 'waveimg' must have [flux * wavelength], 'flux_sum' must have [flux]
    // 'result' = waveimg / flux_sum   must have [wavelength]

    Ok(IOValue::Image(WcsArray::from_array(Dimensioned::new(
        result,
        Unit::None,
    ))))
}

fn run_minmax(image: &WcsArray, start: i64, end: i64, is_min: bool) -> Result<IOValue, IOErr> {
    if !is_min {
        reduce_array_slice(image, start, end, |slices| {
            slices.fold_axis(
                Axis(0),
                -std::f32::INFINITY,
                |x, y| if x > y { *x } else { *y },
            )
        })
    } else {
        reduce_array_slice(image, start, end, |slices| {
            slices.fold_axis(
                Axis(0),
                std::f32::INFINITY,
                |x, y| if x < y { *x } else { *y },
            )
        })
    }
}

fn run_create_argmap(
    image: &WcsArray,
    start: i64,
    end: i64,
    range: i64,
    is_min: bool,
    is_actual_value: bool,
) -> Result<IOValue, IOErr> {
    let start = try_into_unsigned!(start)?;
    let end = try_into_unsigned!(end)?;
    is_sliceable!(image, start, end)?;
    let original_meta = image.meta();
    let original_visualization = image.tag();
    let original_topology = image.topology();
    let mut trypix = 0;
    let mut waveunit = Unit::None;
    for (n, a) in image.meta().clone().unwrap().axes().iter().enumerate() {
        if a.name() == "WAVE" {
            if a.unit() == "Angstrom" || a.unit() == "Ang" {
                trypix = n;
                waveunit = Unit::Custom("Ang".to_string());
            }
        }
    }

    let image_val = image.scalar();

    let slices = image_val.slice_axis(Axis(0), Slice::from(start..end));
    let dim = slices.dim();
    let size = dim.as_array_view();
    let new_size: Vec<_> = size.iter().skip(1).cloned().collect();

    let waveimg = ArrayD::from_shape_fn(new_size, |index| {
        let mut value = if !is_min {
            -std::f32::INFINITY
        } else {
            std::f32::INFINITY
        };
        let mut out = 0.0;
        for (k, slice) in slices.axis_iter(Axis(0)).enumerate() {
            if (!is_min && slice[&index] > value) || (is_min && slice[&index] < value) {
                value = slice[&index];
                if is_actual_value {
                    out = match image.pix2world(trypix, ((k + start) as i64 + range) as f32) {
                        Some(value) => value,
                        None => ((k + start) as i64 + range) as f32,
                    };
                } else {
                    out = ((k + start) as i64 + range) as f32;
                }
            }
        }
        out
    });
    let waveimg = Dimensioned::new(waveimg, waveunit);
    let waveimg = WcsArray::new(
        original_meta.to_owned(), //FIXME: Handle metadata
        waveimg,
        original_visualization.to_owned(),
        original_topology.to_owned(),
    );
    Ok(IOValue::Image(waveimg))
}

fn run_argminmax(image: &WcsArray, start: i64, end: i64, is_min: bool) -> Result<IOValue, IOErr> {
    run_create_argmap(image, start, end, 0, is_min, true)
}

fn run_centroid(image: &WcsArray, start: i64, end: i64) -> Result<IOValue, IOErr> {
    let start = try_into_unsigned!(start)?;
    let end = try_into_unsigned!(end)?;
    is_sliceable!(image, start, end)?;
    let original_meta = image.meta();
    let original_visualization = image.tag();
    let original_topology = image.topology();
    let mut trypix = 0;
    let mut waveunit = Unit::None;
    for (n, a) in image.meta().clone().unwrap().axes().iter().enumerate() {
        if a.name() == "WAVE" {
            if a.unit() == "Angstrom" || a.unit() == "Ang" {
                trypix = n;
                waveunit = Unit::Custom("Ang".to_string());
            }
        }
    }

    let image_val = image.scalar();

    let slices = image_val.slice_axis(Axis(0), Slice::from(start..end));
    let dim = slices.dim();
    let size = dim.as_array_view();
    let new_size: Vec<_> = size.iter().skip(1).cloned().collect();
    let new_size_2 = new_size.clone();

    let flux_sum = ArrayD::from_shape_fn(new_size, |index| {
        let mut out = 0.0;
        for (_, slice) in slices.axis_iter(Axis(0)).enumerate() {
            let flux = slice[&index];
            out += flux;
        }
        out
    });
    let flux_sum = Dimensioned::new(flux_sum, image.array().unit().to_owned());
    let flux_sum = WcsArray::new(
        original_meta.to_owned(),
        flux_sum,
        original_visualization.to_owned(),
        original_topology.to_owned(),
    );

    let waveimg = ArrayD::from_shape_fn(new_size_2, |index| {
        let mut out = 0.0;
        for (k, slice) in slices.axis_iter(Axis(0)).enumerate() {
            let flux = slice[&index];
            let wavelength = match image.pix2world(trypix, (k + start) as f32) {
                Some(value) => value,
                None => (k + start) as f32,
            };
            out += flux * wavelength;
        }
        out
    });
    let waveimg = Dimensioned::new(waveimg, image.array().unit().to_owned().mul(waveunit));
    let waveimg = WcsArray::new(
        original_meta.to_owned(), //FIXME: Handle metadata
        waveimg,
        original_visualization.to_owned(),
        original_topology.to_owned(),
    );
    let result = &waveimg / &flux_sum;
    Ok(IOValue::Image(result))
}

fn run_centroid_with_mask(
    image: &WcsArray,
    start_mask: &WcsArray,
    end_mask: &WcsArray,
) -> Result<IOValue, IOErr> {
    let image_val = image.scalar();
    let start_mask_val = start_mask.scalar();
    let end_mask_val = end_mask.scalar();
    let dim = image_val.dim();
    let size = dim.as_array_view();
    let new_size: Vec<_> = size.iter().skip(1).cloned().collect();
    let new_size2 = new_size.clone();
    let flux_sum = ArrayD::from_shape_fn(new_size, |index| {
        let mut out = 0.0;
        let start = start_mask_val[&index] as usize;
        let end = end_mask_val[&index] as usize;
        let slices = image_val.slice_axis(Axis(0), Slice::from(start..end));

        for (_, slice) in slices.axis_iter(Axis(0)).enumerate() {
            let flux = slice[&index];
            out += flux;
        }
        out
    });

    let waveimg = ArrayD::from_shape_fn(new_size2, |index| {
        let mut out = 0.0;
        let start = start_mask_val[&index] as usize;
        let end = end_mask_val[&index] as usize;
        let slices = image_val.slice_axis(Axis(0), Slice::from(start..end));
        for (k, slice) in slices.axis_iter(Axis(0)).enumerate() {
            let flux = slice[&index];
            let wavelength = match image.pix2world(2, (k + start) as f32) {
                Some(value) => value,
                None => (k + start) as f32,
            };
            out += flux * wavelength;
        }
        out
    });

    let result = waveimg / flux_sum;

    // FIXME: Unit support
    // unit of index(Axis 0) should be adobped
    //
    // in above program...
    // 'waveimg' must have [flux * wavelength], 'flux_sum' must have [flux]
    // 'result' = waveimg / flux_sum   must have [wavelength]

    Ok(IOValue::Image(WcsArray::from_array(Dimensioned::new(
        result,
        Unit::None,
    ))))
}

fn run_create_equivalent_width(
    i_off: &WcsArray,
    i_on: &WcsArray,
    fl: f32,
    max: f32,
    is_emission: bool,
) -> Result<IOValue, IOErr> {
    let fl = Dimensioned::new(fl, Unit::Custom("Ang".to_string()));
    let mut t = (i_off - i_on) * fl / i_off * (if is_emission { -1.0 } else { 1.0 });
    let result_val = t.scalar_mut().map(|v| if *v > max { 0.0 } else { *v });
    let result = WcsArray::from_array(Dimensioned::new(result_val, t.array().unit().to_owned()));
    Ok(IOValue::Image(result))
}

fn run_convert_to_logscale(
    image: &WcsArray,
    a: f32,
    v_min: f32,
    v_max: f32,
) -> Result<IOValue, IOErr> {
    let mut out = image.clone();
    out.scalar_mut().par_iter_mut().for_each(|v| {
        let d = (*v - v_min) / (v_max - v_min);
        *v = (a * d + 1.0).ln() / a.ln();
    });

    Ok(IOValue::Image(out))
}

fn run_log10(image: &WcsArray) -> Result<IOValue, IOErr> {
    let mut out = image.clone();
    out.scalar_mut().par_iter_mut().for_each(|v| *v = v.log10());
    Ok(IOValue::Image(out))
}

fn run_image_multiplier(
    i1: &WcsArray,
    i2: &WcsArray,
    coef1: f32,
    coef2: f32,
) -> Result<IOValue, IOErr> {
    are_same_dim!(i1, i2)?;
    let mut i1 = i1.clone();
    let mut i2 = i2.clone();
    //FIXME: Unit data lost
    i1.scalar_mut()
        .par_iter_mut()
        .for_each(|v| *v = (*v).powf(coef1));
    i2.scalar_mut()
        .par_iter_mut()
        .for_each(|v| *v = (*v).powf(coef2));
    let out = i1 * i2;

    Ok(IOValue::Image(out))
}

fn run_negation(image: &WcsArray) -> Result<IOValue, IOErr> {
    let mut out = image.clone();
    out.scalar_mut().par_iter_mut().for_each(|v| *v = -*v);

    Ok(IOValue::Image(out))
}

fn run_gaussian_mean(image: &WcsArray, start: i64, end: i64) -> Result<IOValue, IOErr> {
    let start = try_into_unsigned!(start)?;
    let end = try_into_unsigned!(end)?;
    let mut flag = false;

    is_sliceable!(image, start, end)?;

    let image_val = image.scalar();

    let slices = image_val.slice_axis(Axis(0), Slice::from(start..end));
    let dim = slices.dim();
    let size = dim.as_array_view();
    let new_size: Vec<_> = size.iter().skip(1).cloned().collect();

    let img = ArrayD::from_shape_fn(new_size, |index| {
        let mut sums = vec![0.0, 0.0, 0.0, 0.0];
        let mut lns = vec![0.0, 0.0, 0.0];
        let n = end - start + 1;
        // Caruanas Algorithm
        for (k, slice) in slices.axis_iter(Axis(0)).enumerate() {
            let y = slice[&index];
            let x = k as f32;
            sums[0] += x;
            sums[1] += x * x;
            sums[2] += x * x * x;
            sums[3] += x * x * x * x;
            lns[0] += y.ln();
            lns[1] += x * y.ln();
            lns[2] += x * x * y.ln();
        }
        let a = Matrix3::new(
            n as f32, sums[0], sums[1], sums[0], sums[1], sums[2], sums[1], sums[2], sums[3],
        );
        let b = Vector3::new(lns[0], lns[1], lns[2]);
        let decomp = a.lu();
        let x = decomp.solve(&b);
        let mut sol = Vector3::from([0.0, 0.0, 0.0]);
        match x {
            Some(vector) => sol = vector,
            None => flag = true,
        };
        let _a = &sol[0]; //not used this time.
        let b = &sol[1];
        let c = &sol[2];
        let mean = -b / (2.0 * c) + start as f32;
        let out = match image.pix2world(2, mean as f32) {
            Some(value) => value,
            None => (mean + start as f32),
        };
        out
    });
    match flag {
        // maybe some IOErr enum (presenting computation failure)is necessary
        true => Err(IOErr::UnexpectedInput("Linear algebra failed.".to_string())),
        false => Ok(IOValue::Image(WcsArray::from_array(Dimensioned::new(
            img,
            Unit::None,
        )))),
    }
}

fn run_gaussian_mean_with_mask(
    image: &WcsArray,
    start_mask: &WcsArray,
    end_mask: &WcsArray,
) -> Result<IOValue, IOErr> {
    let image_val = image.scalar();
    let start_mask_val = start_mask.scalar();
    let end_mask_val = end_mask.scalar();
    let dim = image_val.dim();
    let size = dim.as_array_view();
    let new_size: Vec<_> = size.iter().skip(1).cloned().collect();
    let mut flag = false;
    let img = ArrayD::from_shape_fn(new_size, |index| {
        let mut sums = vec![0.0, 0.0, 0.0, 0.0];
        let mut lns = vec![0.0, 0.0, 0.0];
        let start = start_mask_val[&index] as usize;
        let end = end_mask_val[&index] as usize;
        let slices = image_val.slice_axis(Axis(0), Slice::from(start..end));
        let n = end - start + 1;
        // Caruanas Algorithm
        for (k, slice) in slices.axis_iter(Axis(0)).enumerate() {
            let y = slice[&index];
            let x = k as f32;
            sums[0] += x;
            sums[1] += x * x;
            sums[2] += x * x * x;
            sums[3] += x * x * x * x;
            lns[0] += y.ln();
            lns[1] += x * y.ln();
            lns[2] += x * x * y.ln();
        }
        let a = Matrix3::new(
            n as f32, sums[0], sums[1], sums[0], sums[1], sums[2], sums[1], sums[2], sums[3],
        );
        let b = Vector3::new(lns[0], lns[1], lns[2]);
        let decomp = a.lu();
        let x = decomp.solve(&b);
        let mut sol = Vector3::from([0.0, 0.0, 0.0]);
        match x {
            Some(vector) => sol = vector,
            None => flag = true,
        };
        let _a = &sol[0]; //not used this time.
        let b = &sol[1];
        let c = &sol[2];
        let mean = -b / (2.0 * c) + start as f32;
        let out = match image.pix2world(2, mean as f32) {
            Some(value) => value,
            None => (mean + start as f32),
        };
        out
    });
    match flag {
        // maybe some IOErr enum (presenting computation failure)is necessary
        true => Err(IOErr::UnexpectedInput("Linear algebra failed.".to_string())),
        false => Ok(IOValue::Image(WcsArray::from_array(Dimensioned::new(
            img,
            Unit::None,
        )))),
    }
}

fn run_create_emission_line_map(
    i_off: &WcsArray,
    i_on: &WcsArray,
    fl: f32,
    max: f32,
    is_emission: bool,
) -> Result<IOValue, IOErr> {
    let i_off_integral = i_off.scalar();
    let i_on_integral = i_on.scalar();
    let out = (i_off_integral - i_on_integral) * fl * (if is_emission { -1.0 } else { 1.0 });
    let result = out.map(|v| if *v > max { 0.0 } else { *v });

    Ok(IOValue::Image(WcsArray::from_array(Dimensioned::new(
        result,
        Unit::None,
    ))))
}

fn run_change_tag(data_in: &WcsArray, tag: &str) -> Result<IOValue, IOErr> {
    let mut out = data_in.clone();
    out.set_tag(Some(String::from(tag)));

    Ok(IOValue::Image(out))
}

fn run_color_image_to_hsv(image: &WcsArray) -> Result<IOValue, IOErr> {
    dim_is!(image, 3)?;
    let image = image.scalar();
    let mut out = image.clone();
    for (j, slice) in image.axis_iter(Axis(1)).enumerate() {
        for (i, data) in slice.axis_iter(Axis(1)).enumerate() {
            let mut hue = 0.0;
            let max = data[0].max(data[1]).max(data[2]);
            let min = data[0].min(data[1]).min(data[2]);
            if data[0] >= data[1] && data[0] >= data[2] {
                hue = 60.0 * ((data[1] - data[2]) / (max - min));
            } else if data[1] >= data[0] && data[1] >= data[2] {
                hue = 60.0 * ((data[2] - data[0]) / (max - min)) + 120.0;
            } else if data[2] >= data[0] && data[2] >= data[1] {
                hue = 60.0 * ((data[0] - data[1]) / (max - min)) + 240.0;
            } else {
                println!("Unreachable");
            }
            if hue < 0.0 {
                hue += 360.0;
            }
            hue = hue / 360.0 * 65535.0;
            out[[0, j, i]] = hue;
            out[[1, j, i]] = (max - min) / max * 65535.0;
            out[[2, j, i]] = max;
        }
    }
    Ok(IOValue::Image(WcsArray::from_array(Dimensioned::new(
        out,
        Unit::None,
    ))))
}

fn run_lab_to_rgb(image: &WcsArray) -> Result<IOValue, IOErr> {
    dim_is!(image, 3)?;
    let image = image.scalar();
    let mut out = image.clone();
    for (j, slice) in image.axis_iter(Axis(1)).enumerate() {
        for (i, data) in slice.axis_iter(Axis(1)).enumerate() {
            let d = lab::Lab {
                l: data[0],
                a: (std::f32::consts::PI * data[0] / 100.0).sin() * data[1],
                b: (std::f32::consts::PI * data[0] / 100.0).sin() * data[2],
            };
            let c = d.to_rgb();
            out[[0, j, i]] = c[0] as f32;
            out[[1, j, i]] = c[1] as f32;
            out[[2, j, i]] = c[2] as f32;
            println!("d: {:?} -> c: {:?}", d, c);
        }
    }
    Ok(IOValue::Image(WcsArray::from_array(Dimensioned::new(
        out,
        Unit::None,
    ))))
}

fn run_generate_color_image_from_channel(
    image_r: &WcsArray,
    image_g: &WcsArray,
    image_b: &WcsArray,
) -> Result<IOValue, IOErr> {
    dim_is!(image_r, 2)?;
    dim_is!(image_g, 2)?;
    dim_is!(image_b, 2)?;
    are_same_dim!(image_r, image_b)?;
    are_same_dim!(image_b, image_g)?;
    let dim = image_r.scalar().dim();
    let dim = dim.as_array_view();
    let mut colorimage = Vec::with_capacity(3 * dim[0] * dim[1]);
    for &data in image_r.scalar().iter() {
        colorimage.push(data);
    }
    for &data in image_g.scalar().iter() {
        colorimage.push(data);
    }
    for &data in image_b.scalar().iter() {
        colorimage.push(data);
    }
    let img = Array::from_shape_vec((3, dim[0], dim[1]), colorimage).unwrap();

    Ok(IOValue::Image(WcsArray::from_array(Dimensioned::new(
        img.into_dyn(),
        Unit::None,
    ))))
}

fn run_apply_tone_curve(image: &WcsArray, tone_curve: ToneCurveState) -> Result<IOValue, IOErr> {
    let mut image_arr = image.scalar().clone();
    let table = tone_curve.array();
    let table_size = table.len() - 1;
    image_arr.par_map_inplace(|v| {
        let key = (*v * table_size as f32 / 65535.0) as usize;
        let value = 65535.0 * table[key] / table_size as f32;
        *v = value;
    });
    Ok(IOValue::Image(WcsArray::from_array(Dimensioned::new(
        image_arr,
        Unit::None,
    ))))
}

fn run_apply_arcsinh_stretch(image: &WcsArray, beta: f32) -> Result<IOValue, IOErr> {
    dim_is!(image, 3)?;
    let tag = image.tag();
    let image = image.scalar();
    let mut out = image.clone();
    for (j, slice) in image.axis_iter(Axis(1)).enumerate() {
        for (i, data) in slice.axis_iter(Axis(1)).enumerate() {
            let l = (data[0] + data[1] + data[2]) / 3.0;
            let s_fact = libm::asinh((l * beta) as f64) / (l as f64 * libm::asinh(beta as f64));
            let s_fact = s_fact as f32;
            out[[0, j, i]] = s_fact * data[0];
            out[[1, j, i]] = s_fact * data[1];
            out[[2, j, i]] = s_fact * data[2];
        }
    }
    Ok(IOValue::Image(WcsArray::from_array_and_tag(
        Dimensioned::new(out, Unit::None),
        tag.to_owned(),
    )))
}

fn run_down_sampling(image: &WcsArray, n: i64) -> Result<IOValue, IOErr> {
    precheck!(
        n > 0,
        "{}",
        format!("n should be greater than 0, {} > 0", n)
    )?;
    let tag = image.tag();
    if image.scalar().ndim() == 2 {
        let dim = image.scalar().dim();
        let size = dim.as_array_view();
        let mut out = image.clone();
        let mut new_size = ((size[0]) / (n as usize), (size[1]) / (n as usize));
        if size[0] % (n as usize) != 0 {
            new_size.0 += 1;
        }
        if size[1] % (n as usize) != 0 {
            new_size.1 += 1;
        }
        let mut new_image = Vec::with_capacity(new_size.0 * new_size.1);
        let mut counter = 0;
        for v in out.scalar_mut().iter_mut() {
            let coord = (counter % size[1], counter / size[1]);
            if coord.0 % (n as usize) == 0 && coord.1 % (n as usize) == 0 {
                new_image.push(*v);
            }
            counter += 1;
        }

        let out = Array::from_shape_vec(new_size.strides((new_size.1, 1)), new_image).unwrap();
        Ok(IOValue::Image(WcsArray::from_array_and_tag(
            Dimensioned::new(out.into_dyn(), Unit::None),
            tag.to_owned(),
        )))
    } else if image.scalar().ndim() == 3 {
        let dim = image.scalar().dim();
        let size = dim.as_array_view();
        let mut new_size = (size[0], (size[1]) / (n as usize), (size[2]) / (n as usize));
        if size[1] % (n as usize) != 0 {
            new_size.1 += 1;
        }
        if size[2] % (n as usize) != 0 {
            new_size.2 += 1;
        }
        let mut new_image = Vec::with_capacity(new_size.0 * new_size.1 * new_size.2);
        for slice in image.scalar().axis_iter(Axis(0)) {
            let mut counter = 0;
            for v in slice.iter() {
                let coord = (counter % size[2], counter / size[2]);
                if coord.0 % (n as usize) == 0 && coord.1 % (n as usize) == 0 {
                    new_image.push(*v);
                }
                counter += 1;
            }
        }
        let out = Array::from_shape_vec(
            new_size.strides((new_size.1 * new_size.2, new_size.2, 1)),
            new_image,
        )
        .unwrap();
        Ok(IOValue::Image(WcsArray::from_array_and_tag(
            Dimensioned::new(out.into_dyn(), Unit::None),
            tag.to_owned(),
        )))
    } else {
        Err(IOErr::UnexpectedInput(format!(
            "This is neither 2 nor 3 dimensional image!",
        )))
    }
}

#[cfg(test)]
mod test {
    use super::{run_fits_to_image, run_make_plane3d, run_open_fits, run_slice_3d_to_2d, IOValue};
    use crate::PATHS;
    use std::path::PathBuf;
    #[test]
    fn test_open_fits() {
        let path = PATHS::FileList(vec![PathBuf::from("test/test.fits")]);
        if let PATHS::FileList(path) = path {
            if let IOValue::Fits(fits) = run_open_fits(path.to_vec(), 0).unwrap() {
                if let IOValue::Image(image) = run_fits_to_image(&fits, 0, "").unwrap() {
                    if let IOValue::Map2dTo3dCoords(map) = run_make_plane3d(
                        &[0.0, 0.0, 0.0],
                        &[1.0, 0.5, 0.0],
                        &[0.0, 0.5, 1.0],
                        10,
                        20,
                    )
                    .unwrap()
                    {
                        let _sliced_image = run_slice_3d_to_2d(&image, &map);
                        return;
                    }
                }
            }
        }
        panic!("Failed somewhere!");
    }
}
