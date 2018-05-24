use std::borrow::Cow;

use glium::texture::{ClientFormat, RawImage2d};
use ndarray::Array2;

use super::{Error, State};

pub fn get_size(original_size: (u32, u32)) -> (f32, f32) {
    const ZOOM: f32 = 10.0;
    (original_size.0 as f32 * ZOOM, original_size.1 as f32 * ZOOM)
}

pub fn make_raw_image<'a>(image: &Array2<f32>, state: &State) -> Result<RawImage2d<'a, u8>, Error> {
    let size = image.dim();
    let mut data = Vec::with_capacity(4 * size.0 * size.1);

    if !state.vmin.is_nan() && !state.vmax.is_nan() {
        for val in image.iter() {
            // Make data
            let [r, g, b] = state.lut.color_at_bounds(*val, state.vmin, state.vmax);
            data.push(r);
            data.push(g);
            data.push(b);
            data.push(255u8);
        }
        Ok(RawImage2d {
            data: Cow::Owned(data),
            width: size.0 as u32,
            height: size.1 as u32,
            format: ClientFormat::U8U8U8U8,
        })
    } else {
        Err(Error::Msg("vmin, vmax not set"))
    }
}
