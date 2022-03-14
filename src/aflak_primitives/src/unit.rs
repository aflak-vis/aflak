use std::{fmt, ops};

use fitrs::{FitsData, Hdu, HeaderValue, WCS};
use ndarray::{ArrayD, ArrayView1, ArrayView2, IxDyn};
extern crate regex;

use crate::fits::{FitsArrayReadError, FitsDataToArray};

/// A unit of measurement.
///
/// Would like to extend this type to include SI units or frequent units in
/// astrophysics.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Unit {
    /// Unit is unknown
    None,
    /// Custom unit represented with a string
    Custom(String),
}

/// Decompose the unit into elements and have a Hashmap of key (unit name) and exponent values.
/// # Examples
///
/// ```rust
/// extern crate aflak_primitives as primitives;
/// use primitives::{Unit, DerivedUnit};
///
/// let flux_unit = Unit::Custom("erg/s/cm^2/Ang/spaxel".to_owned());
/// let flux_unit_decomposed = DerivedUnit::new(&flux_unit.repr().to_owned());
/// println!("{:?}", flux_unit_decomposed);
/// //DerivedUnit { exp: Some((1.0, 0)), derived: {"spaxel": -1, "s": -1, "Ang": -1, "erg": 1, "cm": -2} }
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct DerivedUnit {
    exp: Option<(f32, isize)>,
    derived: HashMap<String, isize>,
}
impl DerivedUnit {
    pub fn new(s: &String) -> Self {
        let mut ret = HashMap::new();
        let mut retexp = None;
        let exp;
        let dunit;
        if let Some((e, d)) = s.split_once(' ') {
            exp = Some(e);
            dunit = d;
        } else {
            exp = None;
            dunit = s;
        }

        if let Some(exp) = exp {
            let re = regex::Regex::new(r"e|E").unwrap();
            let v = re.splitn(exp, 2).collect::<Vec<_>>();
            if v.len() == 2 {
                let (pre, p) = (v[0].parse::<f32>().unwrap(), v[1].parse::<isize>().unwrap());
                retexp = Some((pre, p));
            }
        } else {
            retexp = Some((1.0, 0));
        }

        for mat in regex::Regex::new(r"[/|*]?[A-Z|a-z]+\^?[0-9]*")
            .unwrap()
            .find_iter(dunit)
        {
            let key;
            let mut s = mat.as_str();
            let mut p = if s.chars().nth(0) == Some('/') {
                s = &s[1..];
                -1
            } else if s.chars().nth(0) == Some('*') {
                s = &s[1..];
                1
            } else {
                1
            };
            if let Some((k, dec)) = s.split_once('^') {
                key = k;
                p *= dec.parse::<isize>().unwrap();
            } else {
                key = s;
            }
            ret.insert(key.to_string(), p);
        }

        DerivedUnit {
            exp: retexp,
            derived: ret,
        }
    }

    pub fn mul(&self, d: DerivedUnit) -> Self {
        let retexp = match (self.exp, d.exp) {
            (Some(lexp), Some(rexp)) => Some(((lexp.0 * rexp.0), (lexp.1 + rexp.1))),
            (Some(lexp), None) => Some(lexp),
            (None, Some(rexp)) => Some(rexp),
            (None, None) => None,
        };
        let mut ret = self.derived.clone();
        for (key, val) in d.derived {
            ret.entry(key).and_modify(|e| *e += val).or_insert(val);
        }

        DerivedUnit {
            exp: retexp,
            derived: ret,
        }
    }

    pub fn div(&self, d: DerivedUnit) -> Self {
        let retexp = match (self.exp, d.exp) {
            (Some(lexp), Some(rexp)) => Some(((lexp.0 / rexp.0), (lexp.1 - rexp.1))),
            (Some(lexp), None) => Some(lexp),
            (None, Some(rexp)) => Some(rexp),
            (None, None) => None,
        };
        let mut ret = self.derived.clone();
        for (key, val) in d.derived {
            ret.entry(key).and_modify(|e| *e -= val).or_insert(-val);
        }

        DerivedUnit {
            exp: retexp,
            derived: ret,
        }
    }

    pub fn to_string(&self) -> String {
        let mut ret = String::new();
        if let Some((pre, p)) = self.exp {
            if !(pre == 1.0 && p == 0) {
                if pre - pre.floor() == 0.0 {
                    ret += format!("{}", pre as i32).as_str();
                } else {
                    ret += format!("{}", pre).as_str();
                }
                ret += format!("E{} ", p).as_str();
            }
        }
        let mut derived_vec: Vec<_> = self.derived.iter().collect();
        derived_vec.sort_by(|a, b| b.1.cmp(a.1));
        let mut count = 0;
        for (key, val) in derived_vec {
            if *val == 0 {
                continue;
            }
            if *val < 0 {
                ret += "/";
            } else if *val > 0 && count > 0 {
                ret += "*";
            }
            ret += key;
            let val = val.abs();
            if val > 1 {
                ret += format!("^{}", val).as_str();
            }
            count += 1;
        }
        ret
    }
}

impl Default for Unit {
    fn default() -> Self {
        Unit::None
    }
}

/// Container for a value with a dimension (i.e. a unit).
///
/// This container implements common operations and keep track of the unit
/// of the resulting value. Unit information is guaranteed to be correct as
/// long as the inner flag `homogeneous` is true.
///
/// # Examples
///
/// ```rust
/// extern crate aflak_primitives as primitives;
/// use primitives::{Dimensioned, Unit};
///
/// let meter = Unit::Custom("m".to_owned());
/// let val1 = Dimensioned::new(1, meter.clone());
/// let val2 = Dimensioned::new(2, meter.clone());
/// let sum = val1 + val2;
/// assert!(*sum.scalar() == 3);
/// assert!(sum.unit() == &meter);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dimensioned<V> {
    value: V,
    unit: Unit,
    homogeneous: bool,
}

/// A *n*-dimensional array of floating point values along with meta-data for
/// units and world-coordinates transfer.
///
/// This is the main data structure used by `aflak_primitives` to represent
/// multi-dimensional astrophysical data.
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WcsArray {
    meta: Option<MetaWcsArray>,
    array: Dimensioned<ArrayD<f32>>,
    visualization: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct MetaWcsArray {
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    // TODO: Handle serialization for WCS
    wcs: WCS,
    axes: [Axis; 4],
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Axis {
    name: Option<String>,
    unit: Unit,
}

impl Axis {
    fn new(name: Option<String>, unit: Unit) -> Self {
        Self { name, unit }
    }

    pub fn name(&self) -> &str {
        self.name.as_ref().map(String::as_ref).unwrap_or("")
    }

    pub fn unit(&self) -> &str {
        self.unit.repr()
    }
}

fn read_unit(hdu: &Hdu, key: &str) -> Unit {
    if let Some(unit) = read_string(hdu, key) {
        Unit::Custom(unit)
    } else {
        Unit::None
    }
}

fn read_string(hdu: &Hdu, key: &str) -> Option<String> {
    if let Some(HeaderValue::CharacterString(string)) = hdu.value(key) {
        Some(string.to_owned())
    } else {
        None
    }
}

fn read_float(hdu: &Hdu, key: &str) -> Option<f64> {
    if let Some(HeaderValue::RealFloatingNumber(value)) = hdu.value(key) {
        Some(value.to_owned())
    } else if let Some(HeaderValue::IntegerNumber(value)) = hdu.value(key) {
        Some(value.to_owned() as f64)
    } else {
        None
    }
}

impl WcsArray {
    /// Make `WcsArray` from `Hdu` found in FITS file.
    pub fn from_hdu(hdu: &Hdu) -> Result<WcsArray, FitsArrayReadError> {
        let data = hdu.read_data();
        let image = match data {
            FitsData::FloatingPoint32(image) => FitsDataToArray::<IxDyn>::to_array(image)?,
            FitsData::FloatingPoint64(image) => FitsDataToArray::<IxDyn>::to_array(image)?,
            FitsData::IntegersI32(image) => FitsDataToArray::<IxDyn>::to_array(image)?,
            FitsData::IntegersU32(image) => FitsDataToArray::<IxDyn>::to_array(image)?,
            FitsData::Characters(_) => {
                return Err(FitsArrayReadError::UnsupportedData("Characters"));
            }
        };

        let vunit = read_unit(hdu, "BUNIT");
        let cunit1 = read_unit(hdu, "CUNIT1");
        let cunit2 = read_unit(hdu, "CUNIT2");
        let cunit3 = read_unit(hdu, "CUNIT3");
        let cunit4 = read_unit(hdu, "CUNIT4");
        let ctype1 = read_string(hdu, "CTYPE1");
        let ctype2 = read_string(hdu, "CTYPE2");
        let ctype3 = read_string(hdu, "CTYPE3");
        let ctype4 = read_string(hdu, "CTYPE4");
        let bzeros = read_float(hdu, "BZERO");
        let bscales = read_float(hdu, "BSCALE");
        let wcs = WCS::new(hdu);
        let bzero = if let Some(bzero) = bzeros {
            bzero as f32
        } else {
            0.0
        };
        let bscale = if let Some(bscale) = bscales {
            bscale as f32
        } else {
            1.0
        };

        Ok(Self {
            meta: Some(MetaWcsArray {
                wcs,
                axes: [
                    Axis::new(ctype1, cunit1),
                    Axis::new(ctype2, cunit2),
                    Axis::new(ctype3, cunit3),
                    Axis::new(ctype4, cunit4),
                ],
            }),
            array: vunit.new(image * bscale + bzero),
            visualization: None,
        })
    }

    /// Convert position `pixel` (in pixel coordinates starting from 0) at axis
    /// number `axis` to world coordinates. Return `None` if necessary metadata
    /// is missing.
    pub fn pix2world(&self, axis: usize, pixel: f32) -> Option<f32> {
        self.meta.as_ref().map(|meta| {
            let mut input = [0.0; 4];
            input[axis] = pixel;
            meta.wcs.pix2world(input)[axis]
        })
    }

    /// Make a new array missing all metadata about axes and world coordinates.
    pub fn from_array(array: Dimensioned<ArrayD<f32>>) -> Self {
        Self {
            meta: None,
            array,
            visualization: None,
        }
    }

    pub fn from_array_and_tag(
        array: Dimensioned<ArrayD<f32>>,
        visualization: Option<String>,
    ) -> Self {
        Self {
            meta: None,
            array,
            visualization,
        }
    }

    /// Get reference to contained *n*-dimensional array.
    pub fn scalar(&self) -> &ArrayD<f32> {
        self.array.scalar()
    }

    /// Get mutable reference to contained *n*-dimensional array.
    pub fn scalar_mut(&mut self) -> &mut ArrayD<f32> {
        self.array.scalar_mut()
    }

    /// Get view to contained 1-dimensional array.
    ///
    /// Panic if contained array is not 1-dimensional.
    pub fn scalar1(&self) -> ArrayView1<f32> {
        let i = self.array.scalar();
        i.slice(s![..])
    }

    /// Get view to contained 2-dimensional array.
    ///
    /// Panic if contained array is not 2-dimensional.
    pub fn scalar2(&self) -> ArrayView2<f32> {
        let i = self.array.scalar();
        i.slice(s![.., ..])
    }

    /// Get reference to contained *n*-dimensional array, with the unit of the
    /// values contained in the array attached.
    pub fn array(&self) -> &Dimensioned<ArrayD<f32>> {
        &self.array
    }

    pub fn axes(&self) -> Option<&[Axis]> {
        self.meta.as_ref().map(|meta| meta.axes.as_ref())
    }

    pub fn tag(&self) -> &Option<String> {
        &self.visualization
    }

    pub fn set_tag(&mut self, tag: Option<String>) {
        self.visualization = tag;
    }

    pub fn wcs(&self) -> Option<&WCS> {
        self.meta.as_ref().map(|meta| &meta.wcs)
    }

    /// Make a slice along the specific `indices` in the array.
    ///
    /// Create a new `WcsArray` containing the provided `array`.
    /// The objective is to have correct metadata for the new `array`. The
    /// new metadata is computed from `indices` and the previous metadata.
    ///
    /// TODO: This method is hard to understand, and is potentional buggy
    /// write-only code.
    pub(crate) fn make_slice(
        &self,
        indices: &[(usize, f32, f32)],
        array: Dimensioned<ArrayD<f32>>,
    ) -> WcsArray {
        let slice_index: Vec<_> = indices.iter().map(|idx| idx.0).collect();
        let new_meta = self.meta.as_ref().map(|meta| {
            let mut wcs = meta.wcs.slice(&slice_index);
            for (i, start, end) in indices {
                wcs = wcs.transform(*i, *start, *end);
            }
            let mut axes = [
                Axis::default(),
                Axis::default(),
                Axis::default(),
                Axis::default(),
            ];
            for (i, _, _) in indices {
                axes[*i] = meta.axes[*i].clone();
            }
            MetaWcsArray { wcs, axes }
        });
        WcsArray {
            meta: new_meta,
            array,
            visualization: None,
        }
    }
}

impl Unit {
    pub fn new<V>(self, value: V) -> Dimensioned<V> {
        Dimensioned {
            value,
            unit: self,
            homogeneous: true,
        }
    }

    pub fn repr(&self) -> &str {
        match *self {
            Unit::None => "",
            Unit::Custom(ref unit) => unit,
        }
    }

    pub fn mul(self, rhs: Unit) -> Self {
        match (self, rhs) {
            (Unit::Custom(s1), Unit::Custom(s2)) => {
                let d1 = DerivedUnit::new(&s1);
                let d2 = DerivedUnit::new(&s2);
                let d = d1.mul(d2);
                let s = d.to_string();
                Unit::Custom(s)
            }
            _ => Unit::None,
        }
    }

    pub fn div(self, rhs: Unit) -> Self {
        match (self, rhs) {
            (Unit::Custom(s1), Unit::Custom(s2)) => {
                let d1 = DerivedUnit::new(&s1);
                let d2 = DerivedUnit::new(&s2);
                let d = d1.div(d2);
                let s = d.to_string();
                Unit::Custom(s)
            }
            _ => Unit::None,
        }
    }
}

impl<V> Dimensioned<V> {
    /// Make a new dimensioned value with given unit.
    pub fn new(value: V, unit: Unit) -> Self {
        unit.new(value)
    }

    /// Get reference to contained scalar value (without unit).
    pub fn scalar(&self) -> &V {
        &self.value
    }

    /// Get mutable reference to contained scalar value (without unit).
    pub fn scalar_mut(&mut self) -> &mut V {
        &mut self.value
    }

    /// Get unit.
    pub fn unit(&self) -> &Unit {
        &self.unit
    }

    /// Make a new dimensioned value with the same unit containing the passed
    /// value.
    pub fn with_new_value<W>(&self, value: W) -> Dimensioned<W> {
        Dimensioned {
            value,
            unit: self.unit.clone(),
            homogeneous: self.homogeneous,
        }
    }
}

impl<V, W> ops::Mul<W> for Dimensioned<V>
where
    V: ops::Mul<W>,
{
    type Output = Dimensioned<<V as ops::Mul<W>>::Output>;

    fn mul(self, rhs: W) -> Self::Output {
        Dimensioned {
            value: self.value * rhs,
            unit: self.unit,
            homogeneous: self.homogeneous,
        }
    }
}

impl<'a, V, W> ops::Mul<W> for &'a Dimensioned<V>
where
    &'a V: ops::Mul<W>,
{
    type Output = Dimensioned<<&'a V as ops::Mul<W>>::Output>;

    fn mul(self, rhs: W) -> Self::Output {
        Dimensioned {
            value: &self.value * rhs,
            unit: self.unit.clone(),
            homogeneous: self.homogeneous,
        }
    }
}

impl<V, W> ops::Div<W> for Dimensioned<V>
where
    V: ops::Div<W>,
{
    type Output = Dimensioned<<V as ops::Div<W>>::Output>;

    fn div(self, rhs: W) -> Self::Output {
        Dimensioned {
            value: self.value / rhs,
            unit: self.unit,
            homogeneous: self.homogeneous,
        }
    }
}

impl<'a, V, W> ops::Div<W> for &'a Dimensioned<V>
where
    &'a V: ops::Div<W>,
{
    type Output = Dimensioned<<&'a V as ops::Div<W>>::Output>;

    fn div(self, rhs: W) -> Self::Output {
        Dimensioned {
            value: &self.value / rhs,
            unit: self.unit.clone(),
            homogeneous: self.homogeneous,
        }
    }
}

impl<V, W> ops::Add<Dimensioned<W>> for Dimensioned<V>
where
    V: ops::Add<W>,
{
    type Output = Dimensioned<<V as ops::Add<W>>::Output>;

    fn add(self, rhs: Dimensioned<W>) -> Self::Output {
        let homogeneous = self.unit == rhs.unit && self.homogeneous && rhs.homogeneous;
        Dimensioned {
            value: self.value + rhs.value,
            unit: self.unit,
            homogeneous,
        }
    }
}

impl<V, W> ops::Sub<Dimensioned<W>> for Dimensioned<V>
where
    V: ops::Sub<W>,
{
    type Output = Dimensioned<<V as ops::Sub<W>>::Output>;

    fn sub(self, rhs: Dimensioned<W>) -> Self::Output {
        let homogeneous = self.unit == rhs.unit && self.homogeneous && rhs.homogeneous;
        Dimensioned {
            value: self.value - rhs.value,
            unit: self.unit,
            homogeneous,
        }
    }
}

impl<'a, V, W> ops::Sub<Dimensioned<W>> for &'a Dimensioned<V>
where
    &'a V: ops::Sub<W>,
{
    type Output = Dimensioned<<&'a V as ops::Sub<W>>::Output>;

    fn sub(self, rhs: Dimensioned<W>) -> Self::Output {
        let homogeneous = self.unit == rhs.unit && self.homogeneous && rhs.homogeneous;
        Dimensioned {
            value: &self.value - rhs.value,
            unit: self.unit.clone(),
            homogeneous,
        }
    }
}

impl<'a, 'b, V, W> ops::Sub<&'b Dimensioned<W>> for &'a Dimensioned<V>
where
    &'a V: ops::Sub<&'b W>,
{
    type Output = Dimensioned<<&'a V as ops::Sub<&'b W>>::Output>;

    fn sub(self, rhs: &'b Dimensioned<W>) -> Self::Output {
        let homogeneous = self.unit == rhs.unit && self.homogeneous && rhs.homogeneous;
        Dimensioned {
            value: &self.value - &rhs.value,
            unit: self.unit.clone(),
            homogeneous,
        }
    }
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Unit::None => "",
                Unit::Custom(ref s) => s,
            }
        )
    }
}

impl ops::Mul<f32> for WcsArray {
    type Output = WcsArray;

    fn mul(self, rhs: f32) -> Self::Output {
        WcsArray {
            meta: self.meta,
            array: self.array * rhs,
            visualization: None,
        }
    }
}

impl<'a> ops::Mul<f32> for &'a WcsArray {
    type Output = WcsArray;

    fn mul(self, rhs: f32) -> Self::Output {
        WcsArray {
            meta: self.meta.clone(),
            array: self.array() * rhs,
            visualization: None,
        }
    }
}

impl ops::Div<f32> for WcsArray {
    type Output = WcsArray;

    fn div(self, rhs: f32) -> Self::Output {
        WcsArray {
            meta: self.meta,
            array: self.array / rhs,
            visualization: None,
        }
    }
}

impl<'a> ops::Div<f32> for &'a WcsArray {
    type Output = WcsArray;

    fn div(self, rhs: f32) -> Self::Output {
        WcsArray {
            meta: self.meta.clone(),
            array: self.array() / rhs,
            visualization: None,
        }
    }
}

impl ops::Add for WcsArray {
    type Output = WcsArray;

    fn add(self, rhs: WcsArray) -> Self::Output {
        let meta = if self.meta == rhs.meta {
            self.meta
        } else {
            None
        };
        WcsArray {
            meta,
            array: self.array + rhs.array,
            visualization: None,
        }
    }
}

impl ops::Sub for WcsArray {
    type Output = WcsArray;

    fn sub(self, rhs: WcsArray) -> Self::Output {
        let meta = if self.meta == rhs.meta {
            self.meta
        } else {
            None
        };
        WcsArray {
            meta,
            array: self.array - rhs.array,
            visualization: None,
        }
    }
}

impl<'a, 'b> ops::Sub<&'b WcsArray> for &'a WcsArray {
    type Output = WcsArray;

    fn sub(self, rhs: &'b WcsArray) -> Self::Output {
        WcsArray {
            meta: self.meta.clone(),
            array: &self.array - &rhs.array,
            visualization: None,
        }
    }
}
