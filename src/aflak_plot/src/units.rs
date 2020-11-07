/// Define transformation between pixel coordinates and world coordinates for
/// an axis.
pub struct AxisTransform<'a, F> {
    label: &'a str,
    unit: &'a str,
    transform: F,
}

impl<'a, F> AxisTransform<'a, F> {
    /// Make a new axis transform with the given `label`, `unit` and `transform`
    /// function.
    pub fn new(label: &'a str, unit: &'a str, transform: F) -> Self {
        Self {
            label,
            unit,
            transform,
        }
    }

    /// Get axis label.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Get axis unit.
    pub fn unit(&self) -> &str {
        &self.unit
    }

    /// Get axis name (label and unit).
    pub fn name(&self) -> String {
        match (self.label, self.unit) {
            ("", "") => String::new(),
            (label, "") => label.to_owned(),
            ("", unit) => unit.to_owned(),
            (label, unit) => format!("{} ({})", label, unit),
        }
    }
}

impl<'a, T> AxisTransform<'a, fn(T) -> T> {
    /// Get the identity transformation. Keep pixel coordinates.
    pub fn id(label: &'a str, unit: &'a str) -> Self {
        fn id<T>(x: T) -> T {
            x
        }
        Self {
            label,
            unit,
            transform: id,
        }
    }

    /// Convenience function to get a None value.
    pub fn none() -> Option<&'static Self> {
        None
    }
}

impl<'a, F: Fn(f32) -> f32> AxisTransform<'a, F> {
    /// Convert pixel to world coordinates.
    pub fn pix2world(&self, p: f32) -> f32 {
        (self.transform)(p)
    }
}
