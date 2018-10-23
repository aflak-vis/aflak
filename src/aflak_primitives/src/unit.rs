use std::ops;

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

impl Unit {
    pub fn new<V>(self, value: V) -> Dimensioned<V> {
        Dimensioned {
            value,
            unit: self,
            homogeneous: true,
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
