pub struct AxisTransform<F> {
    unit: String,
    transform: F,
}

impl<F> AxisTransform<F> {
    pub fn new(unit: String, transform: F) -> Self {
        Self { unit, transform }
    }

    pub fn unit(&self) -> &str {
        &self.unit
    }
}

impl<F: Fn(f32) -> f32> AxisTransform<F> {
    pub fn pix2world(&self, p: f32) -> f32 {
        (self.transform)(p)
    }
}
