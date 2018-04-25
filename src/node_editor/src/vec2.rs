use imgui::ImVec2;
use std::ops::{Add, Mul, Sub};

#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize)]
pub struct Vec2(pub f32, pub f32);

impl From<(f32, f32)> for Vec2 {
    fn from(vec: (f32, f32)) -> Self {
        Vec2(vec.0, vec.1)
    }
}

impl From<Vec2> for ImVec2 {
    fn from(vec: Vec2) -> Self {
        Self::new(vec.0, vec.1)
    }
}

impl Vec2 {
    pub fn new<T: Into<Vec2>>(t: T) -> Self {
        t.into()
    }

    pub fn squared_norm(&self) -> f32 {
        self.0 * self.0 + self.1 * self.1
    }
}

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, other: Vec2) -> Vec2 {
        Vec2(self.0 + other.0, self.1 + other.1)
    }
}

impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, other: Vec2) -> Vec2 {
        Vec2(self.0 - other.0, self.1 - other.1)
    }
}

impl<T> Mul<T> for Vec2
where
    T: Into<f32>,
{
    type Output = Vec2;
    fn mul(self, other: T) -> Vec2 {
        let other = other.into();
        Vec2(self.0 * other, self.1 * other)
    }
}
