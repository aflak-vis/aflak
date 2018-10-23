use std::{fmt, ops};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum SiBaseUnit {
    Metre,
    Kilogram,
    Second,
    Ampere,
    Kelvin,
    Mole,
    Candela,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct SiComposedUnit([isize; 7]);
pub const DIMENSIONLESS: SiComposedUnit = SiComposedUnit([0; 7]);

pub trait Unit: Sized {
    fn composed_unit(self) -> SiComposedUnit;

    fn new<V>(self, value: V) -> Dimensioned<V> {
        Dimensioned {
            value,
            unit: self.composed_unit(),
            homogeneous: true,
        }
    }

    fn reverse(self) -> SiComposedUnit {
        let mut unit = self.composed_unit();
        for i in 0..7 {
            unit.0[i] = -unit.0[i];
        }
        unit
    }
}

impl Unit for SiBaseUnit {
    fn composed_unit(self) -> SiComposedUnit {
        let mut base = [0; 7];
        base[self as usize] = 1;
        SiComposedUnit(base)
    }
}

impl Unit for SiComposedUnit {
    fn composed_unit(self) -> SiComposedUnit {
        self
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Dimensioned<V> {
    value: V,
    unit: SiComposedUnit,
    homogeneous: bool,
}

impl<V> Dimensioned<V> {
    pub fn new<U: Unit>(value: V, unit: U) -> Self {
        unit.new(value)
    }

    pub fn scalar(&self) -> &V {
        &self.value
    }

    pub fn unit(&self) -> SiComposedUnit {
        self.unit
    }

    pub fn homogeneous(&self) -> bool {
        self.homogeneous
    }
}

impl<V, W> ops::Mul<Dimensioned<W>> for Dimensioned<V>
where
    V: ops::Mul<W>,
{
    type Output = Dimensioned<<V as ops::Mul<W>>::Output>;

    fn mul(self, rhs: Dimensioned<W>) -> Self::Output {
        Dimensioned {
            value: self.value * rhs.value,
            unit: self.unit * rhs.unit,
            homogeneous: self.homogeneous && rhs.homogeneous,
        }
    }
}

pub trait ScalarOperand {}
impl ScalarOperand for bool {}
impl ScalarOperand for i8 {}
impl ScalarOperand for u8 {}
impl ScalarOperand for i16 {}
impl ScalarOperand for u16 {}
impl ScalarOperand for i32 {}
impl ScalarOperand for u32 {}
impl ScalarOperand for i64 {}
impl ScalarOperand for u64 {}
impl ScalarOperand for i128 {}
impl ScalarOperand for u128 {}
impl ScalarOperand for isize {}
impl ScalarOperand for usize {}
impl ScalarOperand for f32 {}
impl ScalarOperand for f64 {}

impl<V, W> ops::Mul<W> for Dimensioned<V>
where
    V: ops::Mul<W>,
    W: ScalarOperand,
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

impl<V, W> ops::Div<Dimensioned<W>> for Dimensioned<V>
where
    V: ops::Div<W>,
{
    type Output = Dimensioned<<V as ops::Div<W>>::Output>;

    fn div(self, rhs: Dimensioned<W>) -> Self::Output {
        Dimensioned {
            value: self.value / rhs.value,
            unit: self.unit / rhs.unit,
            homogeneous: self.homogeneous && rhs.homogeneous,
        }
    }
}

impl<V, W> ops::Div<W> for Dimensioned<V>
where
    V: ops::Div<W>,
    W: ScalarOperand,
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

impl ops::Mul for SiBaseUnit {
    type Output = SiComposedUnit;

    fn mul(self, rhs: SiBaseUnit) -> Self::Output {
        self.composed_unit() * rhs.composed_unit()
    }
}

impl ops::Mul for SiComposedUnit {
    type Output = SiComposedUnit;

    fn mul(mut self, rhs: SiComposedUnit) -> Self::Output {
        for i in 0..7 {
            self.0[i] += rhs.0[i];
        }
        SiComposedUnit(self.0)
    }
}

impl ops::Div for SiBaseUnit {
    type Output = SiComposedUnit;

    fn div(self, rhs: SiBaseUnit) -> Self::Output {
        self.composed_unit() / rhs.composed_unit()
    }
}

impl ops::Div for SiComposedUnit {
    type Output = SiComposedUnit;

    fn div(mut self, rhs: SiComposedUnit) -> Self::Output {
        for i in 0..7 {
            self.0[i] -= rhs.0[i];
        }
        SiComposedUnit(self.0)
    }
}

impl fmt::Display for SiBaseUnit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use SiBaseUnit::*;
        let repr = match *self {
            Metre => "m",
            Kilogram => "kg",
            Second => "s",
            Ampere => "A",
            Kelvin => "K",
            Mole => "mol",
            Candela => "cd",
        };
        write!(f, "{}", repr)
    }
}

impl fmt::Display for SiComposedUnit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use SiBaseUnit::*;
        const UNITS: &[SiBaseUnit] = &[Metre, Kilogram, Second, Ampere, Kelvin, Mole, Candela];

        let mut first = true;
        for u in UNITS {
            let exp = self.0[*u as usize];
            if exp > 0 {
                if !first {
                    write!(f, ".")?;
                }
                first = false;
                write!(f, "{}", u)?;
                if exp > 1 {
                    write!(f, "^{}", exp)?;
                }
            }
        }
        for u in UNITS {
            let exp = self.0[*u as usize];
            if exp < 0 {
                if first {
                    write!(f, "1")?;
                }
                write!(f, "/")?;
                first = false;
                write!(f, "{}", u)?;
                if exp > 1 {
                    write!(f, "^{}", -exp)?;
                }
            }
        }
        Ok(())
    }
}

impl<V> fmt::Display for Dimensioned<V>
where
    V: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.homogeneous {
            if self.unit == DIMENSIONLESS {
                write!(f, "{}", self.value)
            } else {
                write!(f, "{} {}", self.value, self.unit)
            }
        } else {
            write!(f, "{} !", self.value)
        }
    }
}

impl<V, W> ops::Add<Dimensioned<W>> for Dimensioned<V>
where
    V: ops::Add<W>,
{
    type Output = Dimensioned<<V as ops::Add<W>>::Output>;

    fn add(self, rhs: Dimensioned<W>) -> Self::Output {
        Dimensioned {
            value: self.value + rhs.value,
            unit: self.unit,
            homogeneous: self.unit == rhs.unit && self.homogeneous && rhs.homogeneous,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SiBaseUnit, Unit, DIMENSIONLESS};
    #[test]
    fn coulomb() {
        let one_second = SiBaseUnit::Second.new(1);
        let one_ampere = SiBaseUnit::Ampere.new(1);
        let one_coulomb = (SiBaseUnit::Second * SiBaseUnit::Ampere).new(1);
        assert_eq!(one_second * one_ampere, one_coulomb)
    }

    #[test]
    fn speed() {
        let one_metre = SiBaseUnit::Metre.new(1);
        let one_second = SiBaseUnit::Second.new(1);
        let one_metre_per_second = (SiBaseUnit::Metre / SiBaseUnit::Second).new(1);
        assert_eq!(one_metre / one_second, one_metre_per_second)
    }

    #[test]
    fn display_metre() {
        assert_eq!("m", format!("{}", SiBaseUnit::Metre));
    }

    #[test]
    fn display_metre_per_second() {
        assert_eq!("m/s", format!("{}", SiBaseUnit::Metre / SiBaseUnit::Second));
    }

    #[test]
    fn display_square_metre() {
        assert_eq!("m^2", format!("{}", SiBaseUnit::Metre * SiBaseUnit::Metre));
    }

    #[test]
    fn display_per_second() {
        assert_eq!("1/m", format!("{}", SiBaseUnit::Metre.reverse()));
    }

    #[test]
    fn display_dimensionless() {
        assert_eq!("", format!("{}", DIMENSIONLESS));
    }

    #[test]
    fn display_one_meter_per_second() {
        let one_metre = SiBaseUnit::Metre.new(1);
        let one_second = SiBaseUnit::Second.new(1);
        assert_eq!("1 m/s", format!("{}", one_metre / one_second));
    }

    #[test]
    fn display_one_dimensionless() {
        let one = DIMENSIONLESS.new(1);
        assert_eq!("1", format!("{}", one));
    }

    #[test]
    fn multiply_by_scalar() {
        let one_second = SiBaseUnit::Second.new(1);
        let two_seconds = SiBaseUnit::Second.new(2);
        assert_eq!(two_seconds, one_second * 2);
    }

    #[test]
    fn divide_by_scalar() {
        let one_second = SiBaseUnit::Second.new(1.0);
        let half_second = SiBaseUnit::Second.new(0.5);
        assert_eq!(half_second, one_second / 2.0);
    }

    #[test]
    fn add_homogen_values() {
        let one_second = SiBaseUnit::Second.new(1.0);
        let half_second = SiBaseUnit::Second.new(0.5);
        let one_and_a_half_second = SiBaseUnit::Second.new(1.0 + 0.5);
        assert_eq!(one_second + half_second, one_and_a_half_second);
        assert!((one_second + half_second).homogeneous());
    }

    #[test]
    fn add_non_homogen_values() {
        let one_second = SiBaseUnit::Second.new(1.0);
        let one_metre = SiBaseUnit::Metre.new(1.0);
        assert!(!(one_second + one_metre).homogeneous());
    }

    #[test]
    fn format_non_homogen_values() {
        let one_second = SiBaseUnit::Second.new(1);
        let one_metre = SiBaseUnit::Metre.new(1);
        assert_eq!("2 !", format!("{}", one_second + one_metre));
    }
}
