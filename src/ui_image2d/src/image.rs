use std::borrow::Cow;

use glium::texture::{ClientFormat, RawImage2d};

use super::{Error, State};

pub fn get_size(original_size: (u32, u32)) -> (f32, f32) {
    const ZOOM: f32 = 10.0;
    (original_size.0 as f32 * ZOOM, original_size.1 as f32 * ZOOM)
}

pub fn make_raw_image<'a>(
    image: &Vec<Vec<f32>>,
    state: &State,
) -> Result<RawImage2d<'a, u8>, Error> {
    let height = image.len();
    let mut width = None;
    let mut data = Vec::with_capacity(4 * height * height);

    if !state.vmin.is_nan() && !state.vmax.is_nan() {
        for row in image.iter() {
            // Check width
            let len = row.len();
            if let Some(w) = width {
                if len != w {
                    return Err(Error::Msg("Could not make image: incoherent width"));
                }
            } else {
                width = Some(len);
            }

            // Make data
            for val in row.iter() {
                let [r, g, b] = state.lut.color_at_bounds(*val, state.vmin, state.vmax);
                data.push(r);
                data.push(g);
                data.push(b);
                data.push(255u8);
            }
        }
        if width.is_none() {
            Err(Error::Msg("Could not make image: width is null."))
        } else {
            Ok(RawImage2d {
                data: Cow::Owned(data),
                width: width.unwrap() as u32,
                height: height as u32,
                format: ClientFormat::U8U8U8U8,
            })
        }
    } else {
        Err(Error::Msg("vmin, vmax not set"))
    }
}
