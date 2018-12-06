use std::{fmt, ops};

use fitrs::{FitsData, Hdu, HeaderValue, WCS};
use ndarray::{ArrayD, ArrayView1, ArrayView2, IxDyn};

use fits::{FitsArrayReadError, FitsDataToArray};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Unit {
    None,
    Custom(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dimensioned<V> {
    value: V,
    unit: Unit,
    homogeneous: bool,
}

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
    cunits: [Unit; 4],
}

fn read_unit(hdu: &Hdu, key: &str) -> Unit {
    if let Some(HeaderValue::CharacterString(unit)) = hdu.value(key) {
        Unit::Custom(unit.to_owned())
    } else {
        Unit::None
    }
}

impl WcsArray {
    pub fn from_hdu(hdu: &Hdu) -> Result<WcsArray, FitsArrayReadError> {
        let data = hdu.read_data();
        let image = match *data {
            FitsData::FloatingPoint32(ref image) => FitsDataToArray::<IxDyn>::to_array(image)?,
            FitsData::FloatingPoint64(ref image) => FitsDataToArray::<IxDyn>::to_array(image)?,
            FitsData::IntegersI32(ref image) => FitsDataToArray::<IxDyn>::to_array(image)?,
            FitsData::IntegersU32(ref image) => FitsDataToArray::<IxDyn>::to_array(image)?,
            FitsData::Characters(_) => {
                return Err(FitsArrayReadError::UnsupportedData("Characters"))
            }
        };

        let vunit = read_unit(hdu, "BUNIT");
        let cunit1 = read_unit(hdu, "CUNIT1");
        let cunit2 = read_unit(hdu, "CUNIT2");
        let cunit3 = read_unit(hdu, "CUNIT3");
        let cunit4 = read_unit(hdu, "CUNIT4");
        let wcs = WCS::new(hdu);
        Ok(Self {
            meta: Some(MetaWcsArray {
                wcs,
                cunits: [cunit1, cunit2, cunit3, cunit4],
            }),
            array: vunit.new(image),
        })
    }

    pub fn pix2world(&self, axis: usize, pixel: f32) -> Option<f32> {
        self.meta.as_ref().map(|meta| {
            let mut input = [0.0; 4];
            input[axis] = pixel;
            meta.wcs.pix2world(input)[axis]
        })
    }

    pub fn from_array(array: Dimensioned<ArrayD<f32>>) -> Self {
        Self { meta: None, array }
    }

    pub fn scalar(&self) -> &ArrayD<f32> {
        self.array.scalar()
    }

    pub fn scalar_mut(&mut self) -> &mut ArrayD<f32> {
        self.array.scalar_mut()
    }

    pub fn scalar1(&self) -> ArrayView1<f32> {
        let i = self.array.scalar();
        i.slice(s![..])
    }

    pub fn scalar2(&self) -> ArrayView2<f32> {
        let i = self.array.scalar();
        i.slice(s![.., ..])
    }

    pub fn array(&self) -> &Dimensioned<ArrayD<f32>> {
        &self.array
    }

    pub fn cunits(&self) -> Option<&[Unit]> {
        self.meta.as_ref().map(|meta| meta.cunits.as_ref())
    }

    pub fn wcs(&self) -> Option<&WCS> {
        self.meta.as_ref().map(|meta| &meta.wcs)
    }

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
            let mut cunits = [Unit::None, Unit::None, Unit::None, Unit::None];
            for (i, _, _) in indices {
                cunits[*i] = meta.cunits[*i].clone();
            }
            MetaWcsArray { wcs, cunits }
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
    pub fn new(value: V, unit: Unit) -> Self {
        unit.new(value)
    }

    pub fn scalar(&self) -> &V {
        &self.value
    }

    pub fn scalar_mut(&mut self) -> &mut V {
        &mut self.value
    }

    pub fn unit(&self) -> &Unit {
        &self.unit
    }

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

impl<'a> ops::Mul<f32> for &'a WcsArray {
    type Output = WcsArray;

    fn mul(self, rhs: f32) -> Self::Output {
        WcsArray {
            meta: self.meta.clone(),
            array: self.array() * rhs,
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
