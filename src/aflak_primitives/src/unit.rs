use std::{fmt, ops};

use fitrs::{FitsData, Hdu, HeaderValue, WCS};
use ndarray::{ArrayD, ArrayView1, ArrayView2, IxDyn};

use fits::{FitsArrayReadError, FitsDataToArray};

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
        let wcs = WCS::new(hdu);
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
            array: vunit.new(image),
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
        Self { meta: None, array }
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
        }
    }
}

impl<'a> ops::Mul<f32> for &'a WcsArray {
    type Output = WcsArray;

    fn mul(self, rhs: f32) -> Self::Output {
        WcsArray {
            meta: self.meta.clone(),
            array: self.array() * rhs,
        }
    }
}

impl ops::Div<f32> for WcsArray {
    type Output = WcsArray;

    fn div(self, rhs: f32) -> Self::Output {
        WcsArray {
            meta: self.meta,
            array: self.array / rhs,
        }
    }
}

impl<'a> ops::Div<f32> for &'a WcsArray {
    type Output = WcsArray;

    fn div(self, rhs: f32) -> Self::Output {
        WcsArray {
            meta: self.meta.clone(),
            array: self.array() / rhs,
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
        }
    }
}

impl<'a, 'b> ops::Sub<&'b WcsArray> for &'a WcsArray {
    type Output = WcsArray;

    fn sub(self, rhs: &'b WcsArray) -> Self::Output {
        WcsArray {
            meta: self.meta.clone(),
            array: &self.array - &rhs.array,
        }
    }
}
