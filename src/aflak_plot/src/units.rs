pub struct AxisTransform<'a, F> {
    unit: &'a str,
    transform: F,
}

impl<'a, F> AxisTransform<'a, F> {
    pub fn new(unit: &'a str, transform: F) -> Self {
        Self { unit, transform }
    }

    pub fn unit(&self) -> &str {
        &self.unit
    }
}

impl<'a, T> AxisTransform<'a, fn(T) -> T> {
    pub fn id(unit: &'a str) -> Self {
        fn id<T>(x: T) -> T {
            x
        }
        Self {
            unit,
            transform: id,
        }
    }

    pub fn none() -> Option<&'static Self> {
        None
    }
}

impl<'a, F: Fn(f32) -> f32> AxisTransform<'a, F> {
    pub fn pix2world(&self, p: f32) -> f32 {
        (self.transform)(p)
    }
}
