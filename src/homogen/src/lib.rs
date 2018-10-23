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

pub trait Unit: Sized {
    fn composed_unit(self) -> SiComposedUnit;

    fn new<V>(self, value: V) -> Dimensioned<V> {
        Dimensioned {
            value,
            unit: self.composed_unit(),
        }
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
}

impl<V> Dimensioned<V> {
    pub fn new<U: Unit>(value: V, unit: U) -> Self {
        Self {
            value,
            unit: unit.composed_unit(),
        }
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

#[cfg(test)]
mod tests {
    use super::{SiBaseUnit, Unit};
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
}
