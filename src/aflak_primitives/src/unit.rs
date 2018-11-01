use std::{fmt, ops};

use fitrs::{FitsData, Hdu, HeaderValue, WCS};
use ndarray::{Array1, Array2, Array3};

use super::IOErr;

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
pub struct WcsArray3 {
    meta: Option<MetaWcsArray3>,
    array: Dimensioned<Array3<f32>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct MetaWcsArray3 {
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    // TODO: Handle serialization for WCS
    wcs: WCS,
    cunits: [Unit; 3],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WcsArray2 {
    meta: Option<MetaWcsArray2>,
    array: Dimensioned<Array2<f32>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct MetaWcsArray2 {
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    // TODO: Handle serialization for WCS
    wcs: WCS,
    cunits: [Unit; 2],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WcsArray1 {
    meta: Option<MetaWcsArray1>,
    array: Dimensioned<Array1<f32>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct MetaWcsArray1 {
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    // TODO: Handle serialization for WCS
    wcs: WCS,
    cunit: Unit,
}

impl WcsArray3 {
    pub fn from_hdu(hdu: &Hdu) -> Result<WcsArray3, IOErr> {
        fn read_unit(hdu: &Hdu, key: &str) -> Unit {
            if let Some(HeaderValue::CharacterString(unit)) = hdu.value(key) {
                Unit::Custom(unit.to_owned())
            } else {
                Unit::None
            }
        }

        let data = hdu.read_data();
        let image = match data {
            &FitsData::FloatingPoint32(ref image) => Array3::from_shape_vec(
                (image.shape[2], image.shape[1], image.shape[0]),
                image.data.clone(),
            ).map_err(IOErr::ShapeError)?,
            _ => unimplemented!(),
        };
        let vunit = read_unit(hdu, "BUNIT");
        let cunit1 = read_unit(hdu, "CUNIT1");
        let cunit2 = read_unit(hdu, "CUNIT2");
        let cunit3 = read_unit(hdu, "CUNIT3");
        let wcs = WCS::new(hdu);
        Ok(Self {
            meta: Some(MetaWcsArray3 {
                wcs,
                cunits: [cunit1, cunit2, cunit3],
            }),
            array: vunit.new(image),
        })
    }

    pub fn from_array(array: Dimensioned<Array3<f32>>) -> Self {
        Self { meta: None, array }
    }

    pub fn scalar(&self) -> &Array3<f32> {
        self.array.scalar()
    }

    pub fn array(&self) -> &Dimensioned<Array3<f32>> {
        &self.array
    }

    pub(crate) fn make_slice1(&self, index: usize, array: Dimensioned<Array1<f32>>) -> WcsArray1 {
        let new_meta = self.meta.as_ref().map(|meta| MetaWcsArray1 {
            wcs: meta.wcs.slice(&[index]),
            cunit: meta.cunits[index].clone(),
        });
        WcsArray1 {
            meta: new_meta,
            array,
        }
    }

    pub(crate) fn make_slice2(
        &self,
        index: &[(usize, f32, f32); 2],
        array: Dimensioned<Array2<f32>>,
    ) -> WcsArray2 {
        let slice_index = [index[0].0, index[1].0];
        let new_meta = self.meta.as_ref().map(|meta| MetaWcsArray2 {
            wcs: meta
                .wcs
                .slice(&slice_index)
                .transform(index[0].0, index[0].1, index[0].2)
                .transform(index[1].0, index[1].1, index[1].2),
            cunits: [
                meta.cunits[index[0].0].clone(),
                meta.cunits[index[1].0].clone(),
            ],
        });
        WcsArray2 {
            meta: new_meta,
            array,
        }
    }
}

impl WcsArray2 {
    pub fn from_array(array: Dimensioned<Array2<f32>>) -> Self {
        Self { meta: None, array }
    }

    pub fn scalar(&self) -> &Array2<f32> {
        self.array.scalar()
    }

    pub fn array(&self) -> &Dimensioned<Array2<f32>> {
        &self.array
    }

    pub fn cunits(&self) -> Option<[&Unit; 2]> {
        self.meta
            .as_ref()
            .map(|meta| [&meta.cunits[0], &meta.cunits[1]])
    }

    pub fn wcs(&self) -> Option<&WCS> {
        self.meta.as_ref().map(|meta| &meta.wcs)
    }
}

impl WcsArray1 {
    pub fn from_array(array: Dimensioned<Array1<f32>>) -> Self {
        Self { meta: None, array }
    }

    pub fn scalar(&self) -> &Array1<f32> {
        self.array.scalar()
    }

    pub fn array(&self) -> &Dimensioned<Array1<f32>> {
        &self.array
    }

    pub fn cunit(&self) -> Option<&Unit> {
        self.meta.as_ref().map(|meta| &meta.cunit)
    }

    pub fn wcs(&self) -> Option<&WCS> {
        self.meta.as_ref().map(|meta| &meta.wcs)
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

impl<'a> ops::Mul<f32> for &'a WcsArray1 {
    type Output = WcsArray1;

    fn mul(self, rhs: f32) -> Self::Output {
        WcsArray1 {
            meta: self.meta.clone(),
            array: self.array() * rhs,
        }
    }
}

impl<'a> ops::Mul<f32> for &'a WcsArray2 {
    type Output = WcsArray2;

    fn mul(self, rhs: f32) -> Self::Output {
        WcsArray2 {
            meta: self.meta.clone(),
            array: self.array() * rhs,
        }
    }
}

impl ops::Add for WcsArray1 {
    type Output = WcsArray1;

    fn add(self, rhs: WcsArray1) -> Self::Output {
        let meta = if self.meta == rhs.meta {
            self.meta
        } else {
            None
        };
        WcsArray1 {
            meta,
            array: self.array + rhs.array,
        }
    }
}

impl ops::Add for WcsArray2 {
    type Output = WcsArray2;

    fn add(self, rhs: WcsArray2) -> Self::Output {
        let meta = if self.meta == rhs.meta {
            self.meta
        } else {
            None
        };
        WcsArray2 {
            meta,
            array: self.array + rhs.array,
        }
    }
}
