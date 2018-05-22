use imgui::ImStr;

use std::iter;
use std::slice;

#[derive(Clone, Debug, PartialEq)]
pub struct ColorLUT {
    /// Linear gradient
    /// Takes a series of color stops that indicate how to interpolate between the colors
    gradient: Vec<(f32, [u8; 3])>,
    lims: (f32, f32),
}

#[derive(Copy, Clone, Hash, Debug, PartialEq, Eq)]
pub enum BuiltinLUT {
    Grey,
    GreyClip,
    Thermal,
    Flame,
    Yellowy,
}

impl From<BuiltinLUT> for Vec<(f32, [u8; 3])> {
    fn from(lut: BuiltinLUT) -> Self {
        lut.lut().gradient
    }
}

impl BuiltinLUT {
    pub fn values() -> slice::Iter<'static, Self> {
        use self::BuiltinLUT::*;
        const VALUES: [BuiltinLUT; 5] = [Grey, GreyClip, Thermal, Flame, Yellowy];
        VALUES.into_iter()
    }

    pub fn name(&self) -> &'static ImStr {
        match *self {
            BuiltinLUT::Grey => im_str!("Grey"),
            BuiltinLUT::GreyClip => im_str!("GreyClip"),
            BuiltinLUT::Yellowy => im_str!("Yellowy"),
            BuiltinLUT::Thermal => im_str!("Thermal"),
            BuiltinLUT::Flame => im_str!("Flame"),
        }
    }

    pub fn lut(&self) -> ColorLUT {
        match *self {
            BuiltinLUT::Grey => ColorLUT::linear(vec![(0.0, [0, 0, 0]), (1.0, [255, 255, 255])]),
            BuiltinLUT::GreyClip => ColorLUT::linear(vec![
                (0.0, [0, 0, 0]),
                (0.99, [255, 255, 255]),
                (1.0, [255, 0, 0]),
            ]),
            BuiltinLUT::Yellowy => ColorLUT::linear(vec![
                (0.0, [0, 0, 0]),
                (0.25, [32, 0, 129]),
                (0.5, [115, 15, 255]),
                (0.75, [255, 255, 0]),
                (1.0, [255, 255, 255]),
            ]),
            BuiltinLUT::Thermal => ColorLUT::linear(vec![
                (0.0, [0, 0, 0]),
                (1.0 / 3.0, [185, 0, 0]),
                (2.0 / 3.0, [255, 220, 0]),
                (1.0, [255, 255, 255]),
            ]),
            BuiltinLUT::Flame => ColorLUT::linear(vec![
                (0.0, [0, 0, 0]),
                (0.2, [7, 0, 220]),
                (0.5, [236, 0, 134]),
                (0.8, [246, 246, 0]),
                (1.0, [255, 255, 255]),
            ]),
        }
    }
}

impl ColorLUT {
    /// Create a linear gradient.
    pub fn linear<T: Into<f32>>(colors: Vec<(T, [u8; 3])>) -> ColorLUT {
        let mut vec = Vec::with_capacity(colors.len());
        for (c, color) in colors {
            vec.push((c.into(), color))
        }
        ColorLUT {
            gradient: vec,
            lims: (0.0, 1.0),
        }
    }

    pub fn color_at_bounds(&self, mut point: f32, vmin: f32, vmax: f32) -> [u8; 3] {
        if point < vmin {
            point = vmin;
        } else if point > vmax {
            point = vmax;
        }
        self.color_at((point - vmin) / (vmax - vmin))
    }

    pub fn color_at(&self, point: f32) -> [u8; 3] {
        for ((v1, c1), (v2, c2)) in self.bounds() {
            let dv = v2 - v1;
            if v1 <= point && point <= v2 {
                let [r1, g1, b1] = c1;
                let [r2, g2, b2] = c2;
                return if dv == 0.0 {
                    c1
                } else {
                    let r1 = r1 as f32;
                    let r2 = r2 as f32;
                    let g1 = g1 as f32;
                    let g2 = g2 as f32;
                    let b1 = b1 as f32;
                    let b2 = b2 as f32;
                    let dp = point - v1;
                    let coef = dp / dv;
                    [
                        (r1 + (r2 - r1) * coef) as u8,
                        (g1 + (g2 - g1) * coef) as u8,
                        (b1 + (b2 - b1) * coef) as u8,
                    ]
                };
            }
        }
        [0, 0, 0]
    }

    pub fn bounds(&self) -> iter::Zip<StopIter, iter::Skip<StopIter>> {
        let first_color = StopIter::new(self);
        let next_color = first_color.clone().skip(1);
        first_color.zip(next_color)
    }

    pub fn set_min(&mut self, mut min: f32) {
        if min < 0.0 {
            min = 0.0;
        } else if min > 1.0 {
            min = 1.0;
        }
        if min > self.lims.1 {
            self.lims.1 = min;
        }
        self.lims.0 = min;
    }

    pub fn set_max(&mut self, mut max: f32) {
        if max < 0.0 {
            max = 0.0;
        } else if max > 1.0 {
            max = 1.0;
        }
        if max < self.lims.0 {
            self.lims.0 = max;
        }
        self.lims.1 = max;
    }

    pub fn lims(&self) -> (f32, f32) {
        self.lims
    }

    pub fn set_gradient<G: Into<Vec<(f32, [u8; 3])>>>(&mut self, gradient: G) {
        self.gradient = gradient.into();
    }
}

#[derive(Copy, Clone, Debug)]
pub struct StopIter<'a> {
    lut: &'a ColorLUT,
    i: isize,
}

impl<'a> StopIter<'a> {
    fn new(lut: &'a ColorLUT) -> Self {
        Self { lut, i: -1 }
    }
}

impl<'a> Iterator for StopIter<'a> {
    type Item = (f32, [u8; 3]);
    fn next(&mut self) -> Option<Self::Item> {
        let grad = &self.lut.gradient;
        if self.i == -1 && grad.len() > 0 {
            self.i += 1;
            Some((0.0, grad[0].1))
        } else if self.i == grad.len() as isize && grad.len() > 0 {
            self.i += 1;
            Some((1.0, grad[grad.len() - 1].1))
        } else {
            self.lut.gradient.get(self.i as usize).map(|value| {
                self.i += 1;
                (
                    self.lut.lims.0 + (self.lut.lims.1 - self.lut.lims.0) * value.0,
                    value.1,
                )
            })
        }
    }
}

pub mod util {
    pub fn to_u32_color(c: &[u8; 3]) -> u32 {
        (c[0] as u32) << 0 | (c[1] as u32) << 8 | (c[2] as u32) << 16 | 0xFF << 24
    }

    pub fn invert_color(c: u32) -> u32 {
        0xFFFFFFFF - c + 0xFF000000
    }
}

#[cfg(test)]
mod test {
    use super::ColorLUT;
    #[test]
    fn test_color_at() {
        let lut = ColorLUT::linear(vec![
            (0.0, [0, 0, 255]),
            (0.5, [255, 255, 255]),
            (1.0, [255, 0, 0]),
        ]);
        assert_eq!(lut.color_at(0.0), [0, 0, 255]);
        assert_eq!(lut.color_at(1.0), [255, 0, 0]);
        assert_eq!(lut.color_at(0.5), [255, 255, 255]);
        assert_eq!(lut.color_at(0.25), [127, 127, 255]);
    }

    #[test]
    fn test_bounds() {
        let lut = ColorLUT::linear(vec![
            (0.0, [0, 0, 255]),
            (0.5, [255, 255, 255]),
            (1.0, [255, 0, 0]),
        ]);
        let mut bounds = lut.bounds();
        assert_eq!(
            bounds.next(),
            Some(((0.0, [0, 0, 255]), (0.0, [0, 0, 255])))
        );
        assert_eq!(
            bounds.next(),
            Some(((0.0, [0, 0, 255]), (0.5, [255, 255, 255])))
        );
        assert_eq!(
            bounds.next(),
            Some(((0.5, [255, 255, 255]), (1.0, [255, 0, 0])))
        );
        assert_eq!(
            bounds.next(),
            Some(((1.0, [255, 0, 0]), (1.0, [255, 0, 0])))
        );
        assert_eq!(bounds.next(), None);
    }

    #[test]
    fn test_color_bounds_with_limits() {
        let mut lut = ColorLUT::linear(vec![(0.0, [0, 0, 0]), (1.0, [255, 255, 255])]);
        lut.lims.0 = 0.2;
        lut.lims.1 = 0.9;
        assert_eq!(lut.color_at(0.0), [0, 0, 0]);
        assert_eq!(lut.color_at(0.1), [0, 0, 0]);
        assert_eq!(lut.color_at(0.2), [0, 0, 0]);
        assert_eq!(lut.color_at(0.55), [127, 127, 127]);
        assert_eq!(lut.color_at(0.9), [255, 255, 255]);
        assert_eq!(lut.color_at(0.95), [255, 255, 255]);
        assert_eq!(lut.color_at(1.0), [255, 255, 255]);
    }
}
