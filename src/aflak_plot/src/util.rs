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

pub fn to_u32_color(c: &[u8; 3]) -> u32 {
    (c[0] as u32) << 0 | (c[1] as u32) << 8 | (c[2] as u32) << 16 | 0xFF << 24
}

pub fn invert_color(c: u32) -> u32 {
    0xFFFFFFFF - c + 0xFF000000
}
