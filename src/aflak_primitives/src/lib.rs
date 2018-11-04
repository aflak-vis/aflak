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
pub use unit::{Dimensioned, Unit, WcsArray1, WcsArray2, WcsArray3};

use std::path::{Path, PathBuf};
use std::sync::Arc;

use ndarray::{Array1, Array2};
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
    Image1d(WcsArray1),
    Image2d(WcsArray2),
    Image3d(WcsArray3),
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
            cake_transform!(
                "Integral for 3D Image. Parameters: a, b (a <= b).
Compute Sum[k, {a, b}]image[k]. image[k] is k-th slice of 3D-fits image.",
                integral<IOValue, IOErr>(image: Image3d, coef1: Float, coef2: Float) -> Image2d {
                    vec![run_integral(image, *coef1, *coef2)]
                }
            ),
            cake_transform!(
                "Create Equivalent-Width map from off-band and on-band. 
Parameters i1, i2, onband-width, min.
Compute value = (i1 - i2) *fl / i1. if value < min, value changes to NAN.",
                create_equivalent_width<IOValue, IOErr>(i1:Image2d, i2:Image2d, fl: Float, min: Float) -> Image2d {
                    vec![run_create_equivalent_width(i1, i2, *fl, *min)]
                }
            ),
            cake_transform!(
                "Convert to log-scale. Parameter i(image-silce). Compute log(i).",
                convert_to_logscale<IOValue, IOErr>(i1: Image2d) -> Image2d {
                    vec![run_convert_to_logscale(i1)]
                }
            ),
            cake_transform!(
                "Negation. Parameter i(image-slice). Compute -i.",
                negation<IOValue, IOErr>(i1: Image2d) -> Image2d {
                    vec![run_negation(i1)]
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
    let primary_hdu = fits
        .get_by_name("FLUX")
        .or_else(|| fits.get(0))
        .ok_or_else(|| {
            IOErr::UnexpectedInput(
                "Could not find HDU FLUX nor Primary HDU in FITS file. Is the file valid?"
                    .to_owned(),
            )
        })?;
    WcsArray3::from_hdu(&primary_hdu).map(IOValue::Image3d)
}

/// Slice a 3D image through an arbitrary 2D plane
fn run_slice_3d_to_2d(input_img: &WcsArray3, map: &Array2<[f32; 3]>) -> Result<IOValue, IOErr> {
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
        fn to_option(self) -> Option<f32> {
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
                [dx.to_option(), dy.to_option(), dz.to_option()]
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
                [dx.to_option(), dy.to_option(), dz.to_option()]
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
                (Some(x), Some(y)) => if x.abs() < EPSILON {
                    Some(y)
                } else {
                    Some(x)
                },
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
            let array = input_img.array().with_new_value(array);
            let params = MapReverseParams::new(map);
            // TODO: Update WCS value for well-behaved slices!
            let array = if let Some(axes) = params.sliced_axes() {
                input_img.make_slice2(&axes, array)
            } else {
                WcsArray2::from_array(array)
            };
            IOValue::Image2d(array)
        }).map_err(IOErr::ShapeError)
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

fn run_extract_wave(image: &WcsArray3, roi: &roi::ROI) -> Result<IOValue, IOErr> {
    let image_val = image.scalar();
    let mut wave = Vec::with_capacity(image_val.len());
    for i in 0..image_val.dim().0 {
        let mut res = 0.0;
        for (_, val) in roi.filter(image_val.slice(s![i, .., ..])) {
            res += val;
        }
        wave.push(res);
    }
    Ok(IOValue::Image1d(image.make_slice1(
        2,
        image.array().with_new_value(Array1::from_vec(wave)),
    )))
}

fn run_linear_composition_1d(
    i1: &WcsArray1,
    i2: &WcsArray1,
    coef1: f32,
    coef2: f32,
) -> Result<IOValue, IOErr> {
    let out = i1 * coef1 + i2 * coef2;
    Ok(IOValue::Image1d(out))
}

fn run_linear_composition_2d(
    i1: &WcsArray2,
    i2: &WcsArray2,
    coef1: f32,
    coef2: f32,
) -> Result<IOValue, IOErr> {
    let out = i1 * coef1 + i2 * coef2;
    Ok(IOValue::Image2d(out))
}

fn run_make_float3(f1: f32, f2: f32, f3: f32) -> Result<IOValue, IOErr> {
    Ok(IOValue::Float3([f1, f2, f3]))
}

fn run_integral(im: &Array3<f32>, coef1: f32, coef2: f32) -> Result<IOValue, IOErr> {
    //hard-coded!(image-size) improvement required.
    let mut out = Vec::with_capacity(74*74);
    for z in 0..74 {
        for y in 0..74 {
            let mut val = 0.0;
            for x in (coef1 as usize)..(coef2 as usize) {
                let tmp_val = *im
                    .get([x as usize, y as usize, z as usize])
                    .ok_or_else(|| {
                        IOErr::UnexpectedInput(format!(
                            "Input maps to out of bound pixel!: [{}, {}, {}]",
                            x, y, z
                        ))
                    })?;
                val += tmp_val / (coef2 - coef1).abs();
            }
            out.push(val);
        }
    }

    Array2::from_shape_vec((74, 74), out)
        .map(IOValue::Image2d)
        .map_err(IOErr::ShapeError)
}

fn run_create_equivalent_width(i1: &Array2<f32>, i2: &Array2<f32>, fl: f32, min: f32) -> Result<IOValue, IOErr> {
    let out: Array2<f32> = (i1 - i2) *fl / i1;

    let result = out.map(|v| if v < &min { std::f32::NAN } else { *v });
    Ok(IOValue::Image2d(result))
}

fn run_convert_to_logscale(i1: &Array2<f32>) -> Result<IOValue, IOErr> {
    let mut out_iter = i1.iter();
    let mut min: f32 = 1000.0;
    while let Some(i) = out_iter.next() {
        min = min.min(*i);
    }
    let out = i1.map(|v| (v + min.abs() + 1.0).log10());
    Ok(IOValue::Image2d(out))
}

fn run_negation(i1: &Array2<f32>) -> Result<IOValue, IOErr> {
    let out = i1.map(|v| -v);
    Ok(IOValue::Image2d(out))
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
