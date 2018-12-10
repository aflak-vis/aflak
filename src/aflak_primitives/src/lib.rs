#[macro_use]
extern crate lazy_static;
extern crate variant_name;
#[macro_use]
extern crate variant_name_derive;
#[macro_use]
extern crate aflak_cake as cake;
pub extern crate fitrs;
#[macro_use]
pub extern crate ndarray;
extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate serde_derive;
pub extern crate vo;

mod download;
mod export;
mod fits;
mod roi;
mod sia;
mod unit;

pub use export::ExportError;
pub use roi::ROI;
pub use unit::{Dimensioned, Unit, WcsArray};

use std::error::Error;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ndarray::{Array1, Array2, ArrayD, ArrayViewD, Axis, Dimension, Ix3, Slice};
use variant_name::VariantName;

#[derive(Clone, Debug, VariantName, Serialize, Deserialize)]
pub enum IOValue {
    Integer(i64),
    Float(f32),
    Float2([f32; 2]),
    Float3([f32; 3]),
    Str(String),
    Bool(bool),
    Path(PathBuf),
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    Fits(Arc<fitrs::Fits>),
    Image(WcsArray),
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    VOTable(vo::table::VOTable),
    SiaService(sia::SiaService),
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
            (Bool(b1), Bool(b2)) => b1 == b2,
            (Image(i1), Image(i2)) => i1 == i2,
            (Map2dTo3dCoords(m1), Map2dTo3dCoords(m2)) => m1 == m2,
            (Roi(r1), Roi(r2)) => r1 == r2,
            (Path(p1), Path(p2)) => p1 == p2,
            _ => false,
        }
    }
}

#[derive(Debug)]
pub enum IOErr {
    IoError(io::Error, String),
    FITSErr(String),
    UnexpectedInput(String),
    ShapeError(ndarray::ShapeError, String),
    SIAError(vo::sia::Error),
}

impl fmt::Display for IOErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use IOErr::*;

        match self {
            IoError(e, s) => write!(f, "I/O error! {}. This was caused by '{}'.", s, e),
            FITSErr(s) => write!(f, "FITS-related error! {}", s),
            UnexpectedInput(s) => write!(f, "Unexpected input! {}", s),
            ShapeError(e, s) => write!(f, "Shape error! {}. This was caused by '{}'.", s, e),
            SIAError(e) => e.fmt(f),
        }
    }
}

impl Error for IOErr {
    fn description(&self) -> &str {
        "aflak_primitives::IOErr"
    }
}

pub type SuccessOut = cake::compute::SuccessOut<IOValue>;

lazy_static! {
    pub static ref TRANSFORMATIONS: Vec<cake::Transform<IOValue, IOErr>> = {
        vec![
            cake_transform!(
                "Open FITS file from a Path.",
                open_fits<IOValue, IOErr>(path: Path) -> Fits {
                    vec![run_open_fits(path)]
                }
            ),
            cake_transform!(
                "Query FITS files from a database using Simple Image Access protocol.",
                sia_query<IOValue, IOErr>(service: SiaService = sia::no_service(), pos: Float2) -> VOTable {
                    vec![sia::run_query(service, *pos)]
                }
            ),
            cake_transform!(
                "Get Link to access image from VOTable",
                access_url<IOValue, IOErr>(table: VOTable, i: Integer = 0) -> Str {
                    vec![sia::run_acref_from_record(table, *i)]
                }
            ),
            cake_transform!(
                "Download FITS from the provided link",
                download_fits<IOValue, IOErr>(url: Str = "".to_owned()) -> Fits {
                    vec![sia::run_download_fits(url)]
                }
            ),
            cake_transform!(
                "Extract dataset from FITS file.",
                fits_to_image<IOValue, IOErr>(fits: Fits, hdu_idx: Integer = 0, extension: Str = "".to_owned()) -> Image {
                    vec![run_fits_to_image(fits, *hdu_idx, extension)]
                }
            ),
            cake_transform!(
                "Slice one frame of a n-dimensional dataset turning it into an (n-1)-dimensional dataset.",
                slice_one_frame<IOValue, IOErr>(image: Image, frame: Integer = 0) -> Image {
                    vec![run_slice_one_frame(image, *frame)]
                }
            ),
            cake_transform!(
                "Slice an arbitrary plane through a 3D dataset and return the slice.",
                slice_3d_to_2d<IOValue, IOErr>(image: Image, map: Map2dTo3dCoords) -> Image {
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
                make_plane3d<IOValue, IOErr>(p0: Float3 = [0.0; 3], dir1: Float3 = [0.0, 0.0, 1.0], dir2: Float3 = [0.0, 1.0, 0.0], count1: Integer = 1, count2: Integer = 1) -> Map2dTo3dCoords {
                    vec![run_make_plane3d(p0, dir1, dir2, *count1, *count2)]
                }
            ),
            cake_transform!(
                "Extract waveform from image with the provided region of interest.",
                extract_wave<IOValue, IOErr>(image: Image, roi: Roi = roi::ROI::All) -> Image {
                    vec![run_extract_wave(image, roi)]
                }
            ),
            cake_transform!("Replace all values above or below a threshold in a image with NaN.
Takes two parameters: a threshold and a bool.
If bool value is checked, then replaces the values above the threshold with NaN, else replace the values below the threshold with NaN.",
                clip_image<IOValue, IOErr>(image: Image, threshold: Float = 0.0, above: Bool = false) -> Image {
                    vec![run_clip(image, *threshold, *above)]
                }
            ),
            cake_transform!("Replace all NaN values in image with the provided value.",
                replace_nan_image<IOValue, IOErr>(image: Image, placeholder: Float = 0.0) -> Image {
                    vec![run_replace_nan_image(image, *placeholder)]
                }
            ),
            cake_transform!(
                "Compose 2 vectors. Parameters: u, v, a, b.
Compute a*u + b*v.",
                linear_composition<IOValue, IOErr>(i1: Image, i2: Image, coef1: Float = 1.0, coef2: Float = 1.0) -> Image {
                    vec![run_linear_composition(i1, i2, *coef1, *coef2)]
                }
            ),
            cake_transform!(
                "Make a Float3 from 3 float values.",
                make_float3<IOValue, IOErr>(f1: Float = 0.0, f2: Float = 0.0, f3: Float = 0.0) -> Float3 {
                    vec![run_make_float3(*f1, *f2, *f3)]
                }
            ),
            cake_transform!(
                "Integral for 3D Image. Parameters: a=start, b=end (a <= b).
Compute Sum[k, {a, b}]image[k]. image[k] is k-th slice of 3D-fits image.
Second output contains (a + b) / 2
Third output contains (b - a)",
                integral<IOValue, IOErr>(image: Image, start: Integer = 0, end: Integer = 1) -> Image, Float, Float {
                    let middle = (*start as f32 + *end as f32) / 2.0;
                    let width = *end as f32 - *start as f32;
                    vec![run_integral(image, *start, *end), Ok(IOValue::Float(middle)), Ok(IOValue::Float(width))]
                }
            ),
            cake_transform!(
                "Ratio from bands' center wavelength.
Parameters: z(on-band's center wavelength), z1, z2(off-bands' centerwavelength) (z1 < z < z2).
Compute off_ratio = 1 - (z - z1) / (z2 - z1), off_ratio_2 = 1 - (z2 - z) / (z2 - z1)",
                ratio_from_bands<IOValue, IOErr>(on: Float, off_1: Float, off_2: Float) -> Float, Float {
                    if !(off_1 < on && on < off_2) {
                        use IOErr::UnexpectedInput;
                        let msg = format!(
                            "wrong magnitude correlation ({} < {} < {})",
                            off_1, on, off_2
                        );
                        vec![msg; 2].into_iter().map(|msg| Err(UnexpectedInput(msg))).collect()
                    } else {
                        let off_ratio_1 = (on - off_1) / (off_2 - off_1);
                        let off_ratio_2 = 1.0 - off_ratio_1;
                        vec![Ok(IOValue::Float(off_ratio_2)), Ok(IOValue::Float(off_ratio_1))]
                    }
                }
            ),
            cake_transform!(
                "Average for 3D Image. Parameters: a=start, b=end (a <= b).
Compute (Sum[k, {a, b}]image[k]) / (b - a). image[k] is k-th slice of 3D-fits image.
Second output contains (a + b) / 2
Third output contains (b - a)",
                average<IOValue, IOErr>(image: Image, start: Integer = 0, end: Integer = 1) -> Image, Float, Float {
                    let middle = (*start as f32 + *end as f32) / 2.0;
                    let width = *end as f32 - *start as f32;
                    vec![run_average(image, *start, *end), Ok(IOValue::Float(middle)), Ok(IOValue::Float(width))]
                }
            ),
            cake_transform!(
                "Create Equivalent-Width map from off-band and on-band.
Parameters i1, i2, onband-width, min, is_emission.
Compute value = (i1 - i2) * fl / i1 (if is_emission is true, the sign of this value turns over).
if value > max, value changes to 0.",
                create_equivalent_width<IOValue, IOErr>(i1: Image, i2: Image, fl: Float = 1.0, max: Float = ::std::f32::INFINITY, is_emission: Bool = false) -> Image {
                    vec![run_create_equivalent_width(i1, i2, *fl, *max, *is_emission)]
                }
            ),
            cake_transform!(
                "Convert to log-scale. Parameter: 2D image i, a, v_min, v_max
Compute y = log(ax + 1) / log(a)  (x = (value - v_min) / (v_max - v_min))",
                convert_to_logscale<IOValue, IOErr>(i1: Image, a: Float = 1000.0, v_min: Float, v_max: Float) -> Image {
                    vec![run_convert_to_logscale(i1, *a, *v_min, *v_max)]
                }
            ),
            cake_transform!(
                "Image's min and max value. Parameter: image i.
Compute v_min(first), v_max(second)",
                image_min_max<IOValue, IOErr>(i1: Image) -> Float, Float {
                    let mut min = std::f32::MAX;
                    let mut max = std::f32::MIN;
                    let i1_arr = i1.scalar();

                    for i in i1_arr {
                        min = min.min(*i);
                        max = max.max(*i);
                    }

                    vec![Ok(IOValue::Float(min)), Ok(IOValue::Float(max))]
                }
            ),
            cake_transform!(
                "Negation. Parameter: image i. Compute -i.",
                negation<IOValue, IOErr>(i1: Image) -> Image {
                    vec![run_negation(i1)]
                }
            ),
        ]
    };
}

impl cake::NamedAlgorithms<IOErr> for IOValue {
    fn get_transform(s: &str) -> Option<&'static cake::Transform<IOValue, IOErr>> {
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
            "Float3" => IOValue::Float3([0.0; 3]),
            "Roi" => IOValue::Roi(roi::ROI::All),
            "Str" => IOValue::Str("".to_owned()),
            "Bool" => IOValue::Bool(false),
            "Path" => IOValue::Path(PathBuf::from("/")),
            _ => panic!("Unknown variant name provided: {}.", variant_name),
        }
    }
}

impl cake::EditableVariants for IOValue {
    fn editable_variants() -> &'static [&'static str] {
        &[
            "Integer", "Float", "Float2", "Float3", "Roi", "Str", "Bool", "Path",
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
        cake::ConvertibleVariant {
            from: "Str",
            into: "SiaService",
            f: str_to_siaservice,
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
fn str_to_siaservice(from: &IOValue) -> IOValue {
    if let IOValue::Str(s) = from {
        IOValue::SiaService(vo::sia::SiaService::new(::std::borrow::Cow::Owned(
            s.to_owned(),
        )))
    } else {
        panic!("Unexpected input!")
    }
}

/// Open FITS file
fn run_open_fits<P: AsRef<Path>>(path: P) -> Result<IOValue, IOErr> {
    let path = path.as_ref();
    fitrs::Fits::open(path)
        .map(|fits| IOValue::Fits(Arc::new(fits)))
        .map_err(|err| IOErr::IoError(err, format!("Could not open file {:?}", path)))
}

/// Turn a FITS file into an image
fn run_fits_to_image(
    fits: &Arc<fitrs::Fits>,
    hdu_idx: i64,
    extension: &str,
) -> Result<IOValue, IOErr> {
    let primary_hdu = fits
        .get_by_name(extension)
        .or_else(|| {
            if hdu_idx < 0 {
                None
            } else {
                fits.get(hdu_idx as usize)
            }
        })
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
    if frame_idx < 0 {
        return Err(IOErr::UnexpectedInput(format!(
            "slice_one_frame: frame index must be positive, got {}",
            frame_idx
        )));
    }

    let frame_idx = frame_idx as usize;

    let image_val = input_img.scalar();

    let dim = image_val.dim();
    let ndim = dim.ndim();
    let frame_cnt = if let Some(frame_cnt) = dim.as_array_view().first() {
        *frame_cnt
    } else {
        return Err(IOErr::UnexpectedInput("Empty array".to_owned()));
    };
    if frame_idx >= frame_cnt {
        return Err(IOErr::UnexpectedInput(format!(
            "slice_one_frame: frame index higher than input image's frame count ({} >= {})",
            frame_idx, frame_cnt
        )));
    }

    let out = image_val.index_axis(Axis(0), frame_idx);

    let wrap_with_unit = input_img.make_slice(
        &(0..ndim).map(|i| (i, 0.0, 1.0)).collect::<Vec<_>>(),
        input_img.array().with_new_value(out.to_owned()),
    );

    Ok(IOValue::Image(wrap_with_unit))
}

/// Slice a 3D image through an arbitrary 2D plane
fn run_slice_3d_to_2d(input_img: &WcsArray, map: &Array2<[f32; 3]>) -> Result<IOValue, IOErr> {
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
        .map(|array| {
            let array = array.into_dyn();
            let array = input_img.array().with_new_value(array);
            let params = MapReverseParams::new(map);
            let array = if let Some(axes) = params.sliced_axes() {
                input_img.make_slice(&axes, array)
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
    if count1 <= 0 {
        return Err(IOErr::UnexpectedInput(format!(
            "count1 must be strictly positive. Got {}",
            count1
        )));
    }
    if count2 <= 0 {
        return Err(IOErr::UnexpectedInput(format!(
            "count2 must be strictly positive. Got {}",
            count2
        )));
    }

    let count1 = count1 as usize;
    let count2 = count2 as usize;
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
    let image_val = image.scalar();

    let image3 =
        match image_val.view().into_dimensionality::<Ix3>() {
            Ok(image3) => image3,
            Err(e) => return Err(IOErr::ShapeError(
                e,
                format!(
                    "run_extract_wave: Expected an image of dimension 3 as input but got an image of dimension {}",
                    image_val.ndim()
                ),
            )),
        };

    let mut wave = Vec::with_capacity(image3.len());
    for i in 0..image3.dim().0 {
        let mut res = 0.0;
        for (_, val) in roi.filter(image3.slice(s![i, .., ..])) {
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

fn run_clip(image: &WcsArray, threshold: f32, above: bool) -> Result<IOValue, IOErr> {
    let mut image = image.clone();

    for f in image.scalar_mut().iter_mut() {
        if (above && *f >= threshold) || (!above && *f <= threshold) {
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
    let i1_dim = i1.scalar().dim();
    let i2_dim = i2.scalar().dim();
    if i1_dim != i2_dim {
        return Err(IOErr::UnexpectedInput(format!(
            "linear_composition: Cannot compose arrays of different dimensions (first array has dimension {:?}, while second array has dimension {:?}",
            i1_dim, i2_dim
        )));
    }
    let out = i1 * coef1 + i2 * coef2;
    Ok(IOValue::Image(out))
}

fn run_make_float3(f1: f32, f2: f32, f3: f32) -> Result<IOValue, IOErr> {
    Ok(IOValue::Float3([f1, f2, f3]))
}

fn reduce_array_slice<F>(im: &WcsArray, start: i64, end: i64, f: F) -> Result<IOValue, IOErr>
where
    F: Fn(&ArrayViewD<f32>) -> ArrayD<f32>,
{
    if start < 0 {
        return Err(IOErr::UnexpectedInput(format!(
            "start must be positive, but got {}",
            start
        )));
    }
    if end < 0 {
        return Err(IOErr::UnexpectedInput(format!(
            "end must be positive, but got {}",
            end
        )));
    }
    let start = start as usize;
    let end = end as usize;

    let image_val = im.scalar();
    let frame_cnt = if let Some(frame_cnt) = image_val.dim().as_array_view().first() {
        *frame_cnt
    } else {
        return Err(IOErr::UnexpectedInput(format!(
            "Empty array! Cannot reduce."
        )));
    };

    if end >= frame_cnt {
        return Err(IOErr::UnexpectedInput(format!(
            "end higher than input image's frame count ({} >= {})",
            end, frame_cnt
        )));
    }
    if start >= end {
        return Err(IOErr::UnexpectedInput(format!(
            "start higher than end ({} >= {})",
            start, end
        )));
    }

    let slices = image_val.slice_axis(Axis(0), Slice::from(start..end));
    let raw = f(&slices);
    let ndim = raw.ndim();

    let wrap_with_unit = im.make_slice(
        &(0..ndim).map(|i| (i, 0.0, 1.0)).collect::<Vec<_>>(),
        im.array().with_new_value(raw),
    );

    Ok(IOValue::Image(wrap_with_unit))
}

fn run_integral(im: &WcsArray, start: i64, end: i64) -> Result<IOValue, IOErr> {
    reduce_array_slice(im, start, end, |slices| slices.sum_axis(Axis(0)))
}

fn run_average(im: &WcsArray, start: i64, end: i64) -> Result<IOValue, IOErr> {
    reduce_array_slice(im, start, end, |slices| slices.mean_axis(Axis(0)))
}

fn run_create_equivalent_width(
    i1: &WcsArray,
    i2: &WcsArray,
    fl: f32,
    max: f32,
    is_emission: bool,
) -> Result<IOValue, IOErr> {
    let i1_arr = i1.scalar();
    let i2_arr = i2.scalar();
    let out = (i1_arr - i2_arr) * fl / i1_arr * (if is_emission { -1.0 } else { 1.0 });
    let result = out.map(|v| if *v > max { 0.0 } else { *v });

    // FIXME: Unit support
    Ok(IOValue::Image(WcsArray::from_array(Dimensioned::new(
        result,
        Unit::None,
    ))))
}

fn run_convert_to_logscale(
    i1: &WcsArray,
    a: f32,
    v_min: f32,
    v_max: f32,
) -> Result<IOValue, IOErr> {
    let i1_arr = i1.scalar();
    let x = i1_arr.map(|v| (v - v_min) / (v_max - v_min));
    let out = x.map(|v| (a * v + 1.0).ln() / a.ln());

    // FIXME: Unit support
    Ok(IOValue::Image(WcsArray::from_array(Dimensioned::new(
        out,
        Unit::None,
    ))))
}

fn run_negation(i1: &WcsArray) -> Result<IOValue, IOErr> {
    let i1_arr = i1.scalar();
    let out = i1_arr.map(|v| -v);
    // FIXME: Unit support
    Ok(IOValue::Image(WcsArray::from_array(Dimensioned::new(
        out,
        Unit::None,
    ))))
}

#[cfg(test)]
mod test {
    use super::{run_fits_to_image, run_make_plane3d, run_open_fits, run_slice_3d_to_2d, IOValue};
    #[test]
    fn test_open_fits() {
        let path = "test/test.fits";
        if let IOValue::Fits(fits) = run_open_fits(path).unwrap() {
            if let IOValue::Image(image) = run_fits_to_image(&fits, 0, "").unwrap() {
                if let IOValue::Map2dTo3dCoords(map) =
                    run_make_plane3d(&[0.0, 0.0, 0.0], &[1.0, 0.5, 0.0], &[0.0, 0.5, 1.0], 10, 20)
                        .unwrap()
                {
                    let _sliced_image = run_slice_3d_to_2d(&image, &map);
                    return;
                }
            }
        }
        panic!("Failed somewhere!");
    }
}
