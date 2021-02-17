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

#[macro_use]
pub extern crate ndarray;
extern crate nalgebra;
extern crate ndarray_parallel;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate rawloader;

mod fits;
#[macro_use]
mod precond;
mod roi;
mod unit;

pub use crate::roi::ROI;
pub use crate::unit::{Dimensioned, Unit, WcsArray};

use std::error::Error;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use imgui_tone_curve::ToneCurveState;
use nalgebra::{Matrix3, Vector3};
use ndarray::{Array, Array1, Array2, ArrayD, ArrayViewD, Axis, Dimension, ShapeBuilder, Slice};
use ndarray_parallel::prelude::*;
use variant_name::VariantName;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PATHS {
    FileList(Vec<PathBuf>),
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
    Str(String),
    Bool(bool),
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    Fits(Arc<fitrs::Fits>),
    Image(WcsArray),
    Map2dTo3dCoords(Array2<[f32; 3]>),
    Roi(roi::ROI),
    Paths(PATHS),
    ToneCurve(ToneCurveState),
}

impl PartialEq for IOValue {
    fn eq(&self, val: &Self) -> bool {
        use crate::IOValue::*;
        match (self, val) {
            (Integer(i1), Integer(i2)) => i1 == i2,
            (Float(f1), Float(f2)) => f1 == f2,
            (Float2(f1), Float2(f2)) => f1 == f2,
            (ToneCurve(t1), ToneCurve(t2)) => t1 == t2,
            (Float3(f1), Float3(f2)) => f1 == f2,
            (Str(s1), Str(s2)) => s1 == s2,
            (Bool(b1), Bool(b2)) => b1 == b2,
            (Image(i1), Image(i2)) => i1 == i2,
            (Map2dTo3dCoords(m1), Map2dTo3dCoords(m2)) => m1 == m2,
            (Roi(r1), Roi(r2)) => r1 == r2,
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
                "range specification",
                "04. Extract part of data",
                0, 1, 0,
                range_specification<IOValue, IOErr>(image: Image, start: Integer = 0, end: Integer = 1) -> Image {
                    vec![run_range_specification(image, *start, *end)]
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
                "Create scatterplots from two 1D image data.",
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
                "Calculating log10 of input data.",
                "05. Convert data",
                1, 0, 0,
                log10<IOValue, IOErr>(image: Image) -> Image {
                    vec![run_log10(image)]
                }
            ),
            cake_transform!(
                "Negation. Parameter: image. Compute -i.",
                "05. Convert data",
                1, 0, 0,
                negation<IOValue, IOErr>(image: Image) -> Image {
                    vec![run_negation(image)]
                }
            ),
            cake_transform!(
                "Apply tone curve to image",
                "05. Convert data",
                1, 0, 0,
                apply_tone_curve<IOValue, IOErr>(image: Image, tone_curve: ToneCurve) -> Image {
                    vec![run_apply_tone_curve(image, tone_curve.clone())]
                }
            ),
            cake_transform!(
                "Change the visualization tag to BPT, maybe can also be used to realize more tags",
                "05. Convert data",
                1, 0, 0,
                change_tag<IOValue, IOErr>(image: Image, tag: Str = "".to_owned()) -> Image {
                    vec![run_change_tag(image, tag)]
                }
            ),
            cake_transform!(
                "Compose 2 vectors. Parameters: u, v, a, b.
Compute a*u + b*v.",
                "06. Calculate",
                1, 0, 0,
                linear_composition<IOValue, IOErr>(u: Image, v: Image, a: Float = 1.0, b: Float = 1.0) -> Image {
                    vec![run_linear_composition(u, v, *a, *b)]
                }
            ),
            cake_transform!(
                "Calculating division between two WcsArray(image).",
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
                "Extrude along the wavelength",
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
                "Create scatterplots from two 1D image data.",
                1, 0, 0,
                create_scatter<IOValue, IOErr>(xaxis: Image, yaxis: Image) -> Image{
                    vec![run_create_scatter(xaxis, yaxis)]
                }
            ),
            cake_transform!(
                "Create Equivalent-Width map from off-band and on-band.
Parameters i_off, i_on, onband-width, min, is_emission.
Compute value = (i1 - i2) * fl / i1 (if is_emission is true, the sign of this value turns over).
if value > max, value changes to 0.",
                0, 1, 0,
                create_equivalent_width<IOValue, IOErr>(i_off: Image, i_on: Image, fl: Float = 1.0, max: Float = ::std::f32::INFINITY, is_emission: Bool = false) -> Image {
                    vec![run_create_equivalent_width(i_off, i_on, *fl, *max, *is_emission)]
                }
            ),
            cake_transform!(
                "Convert to log-scale. Parameter: image, a, v_min, v_max.
Compute y = log(ax + 1) / log(a)  (x = (value - v_min) / (v_max - v_min))",
                1, 0, 0,
                convert_to_logscale<IOValue, IOErr>(image: Image, a: Float = 1000.0, v_min: Float, v_max: Float) -> Image {
                    vec![run_convert_to_logscale(image, *a, *v_min, *v_max)]
                }
            ),
            cake_transform!(
                "Convert to log10",
                1, 0, 0,
                log10<IOValue, IOErr>(image: Image) -> Image {
                    vec![run_log10(image)]
                }
            ),
            cake_transform!(
                "Image's min and max value. Parameter: image.
Compute v_min(first), v_max(second)",
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
                    let c = 3e5;
                    let image_arr = image.scalar();
                    let out = image_arr.map(|v| c * (*v - w_0) / w_0);

                    vec![Ok(IOValue::Image(WcsArray::from_array(Dimensioned::new(
                        out,
                        Unit::Custom("km/s".to_string()),
                    ))))]
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
        ]
    };
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

impl cake::DefaultFor for IOValue {
    fn default_for(variant_name: &str) -> Self {
        match variant_name {
            "Integer" => IOValue::Integer(0),
            "Float" => IOValue::Float(0.0),
            "Float2" => IOValue::Float2([0.0; 2]),
            "ToneCurve" => IOValue::ToneCurve(ToneCurveState::default()),
            "Float3" => IOValue::Float3([0.0; 3]),
            "Roi" => IOValue::Roi(roi::ROI::All),
            "Str" => IOValue::Str("".to_owned()),
            "Bool" => IOValue::Bool(false),
            "Paths" => IOValue::Paths(PATHS::FileList(vec![])),
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
            "Roi",
            "Str",
            "Bool",
            "Paths",
            "ToneCurve",
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
        for (_, val) in roi.filterx(image_val.slice(s![i, .., ..])) {
            res += val;
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

fn run_extrude(image: &WcsArray, roi: &roi::ROI) -> Result<IOValue, IOErr> {
    dim_is!(image, 3)?;

    let image_val = image.scalar();
    let wave_size = *image_val.dim().as_array_view().first().unwrap();

    let new_size = (wave_size, roi.datalen());
    let mut result = Vec::with_capacity(wave_size * roi.datalen());

    for i in (0..wave_size).rev() {
        for (_, val) in roi.filterx(image_val.slice(s![i, .., ..])) {
            result.push(val);
        }
    }

    let waveimg = Array::from_shape_vec(new_size.strides((roi.datalen(), 1)), result).unwrap();

    Ok(IOValue::Image(WcsArray::from_array(Dimensioned::new(
        waveimg.into_dyn(),
        Unit::None,
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
                    out = match image.pix2world(2, ((k + start) as i64 + range) as f32) {
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

    // FIXME: Unit support
    // unit of index(Axis 0) should be adobped
    //
    // in above program...
    // unit of variable 'out' should be adopted

    Ok(IOValue::Image(WcsArray::from_array(Dimensioned::new(
        waveimg,
        Unit::None,
    ))))
}

fn run_argminmax(image: &WcsArray, start: i64, end: i64, is_min: bool) -> Result<IOValue, IOErr> {
    run_create_argmap(image, start, end, 0, is_min, true)
}

fn run_centroid(image: &WcsArray, start: i64, end: i64) -> Result<IOValue, IOErr> {
    let start = try_into_unsigned!(start)?;
    let end = try_into_unsigned!(end)?;
    is_sliceable!(image, start, end)?;

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

    let waveimg = ArrayD::from_shape_fn(new_size_2, |index| {
        let mut out = 0.0;
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
    let i_off_arr = i_off.scalar();
    let i_on_arr = i_on.scalar();
    let out = (i_off_arr - i_on_arr) * fl / i_off_arr * (if is_emission { -1.0 } else { 1.0 });
    let result = out.map(|v| if *v > max { 0.0 } else { *v });

    // FIXME: Unit support
    // implementation of &WcsArray / &WcsArray is needed
    //
    // in above program...
    // variable 'fl' is width of on-band, so unit of length should be adopted (e.g. [Ang]).

    Ok(IOValue::Image(WcsArray::from_array(Dimensioned::new(
        result,
        Unit::None,
    ))))
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
    i1.scalar_mut()
        .par_iter_mut()
        .for_each(|v| *v = (*v).powf(coef1));
    i2.scalar_mut()
        .par_iter_mut()
        .for_each(|v| *v = (*v).powf(coef2));
    let out = i1.scalar() * i2.scalar();

    Ok(IOValue::Image(WcsArray::from_array(Dimensioned::new(
        out,
        Unit::None,
    ))))
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
    out.visualization = Some(String::from(tag));

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

fn run_down_sampling(image: &WcsArray, n: i64) -> Result<IOValue, IOErr> {
    dim_is!(image, 2)?;
    precheck!(
        n > 0,
        "{}",
        format!("n should be greater than 0, {} > 0", n)
    )?;
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

    Ok(IOValue::Image(WcsArray::from_array(Dimensioned::new(
        out.into_dyn(),
        Unit::None,
    ))))
}

#[cfg(test)]
mod test {
    use super::{run_fits_to_image, run_make_plane3d, run_open_fits, run_slice_3d_to_2d, IOValue};
    use std::path::PathBuf;
    use PATHS;
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
