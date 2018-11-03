use std::borrow::Cow;

use glium::texture::{ClientFormat, RawImage2d};
use ndarray::Array2;

use super::{Error, State};

pub fn make_raw_image<'a>(image: &Array2<f32>, state: &State) -> Result<RawImage2d<'a, u8>, Error> {
    let (m, n) = image.dim();
    let mut data = Vec::with_capacity(3 * n * m);

    if !state.vmin.is_nan() && !state.vmax.is_nan() {
        for val in image.iter() {
            // Make data
            let [r, g, b] = state.lut.color_at_bounds(*val, state.vmin, state.vmax);
            data.push(r);
            data.push(g);
            data.push(b);
        }
        Ok(RawImage2d {
            data: Cow::Owned(data),
            width: n as u32,
            height: m as u32,
            format: ClientFormat::U8U8U8,
        })
    } else {
        Err(Error::Msg("vmin, vmax not set"))
    }
}
