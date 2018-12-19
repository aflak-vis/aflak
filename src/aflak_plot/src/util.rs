use std::ops::{Add, Mul, Sub};

pub fn clamp<T>(v: T, min: T, max: T) -> T
where
    T: PartialOrd,
{
    if v < min {
        min
    } else if v > max {
        max
    } else {
        v
    }
}

pub fn lerp<V, T>(a: V, b: V, t: T) -> <V as Add>::Output
where
    V: Copy + Add + Sub<Output = V> + Mul<Output = V>,
    T: Into<V>,
{
    a + (b - a) * t.into()
}

pub fn to_u32_color(c: [u8; 3]) -> u32 {
    u32::from(c[0]) | (u32::from(c[1]) << 8) | (u32::from(c[2]) << 16) | 0xFF << 24
}

pub fn invert_color(c: u32) -> u32 {
    0xFFFFFFFF - c + 0xFF000000
}
