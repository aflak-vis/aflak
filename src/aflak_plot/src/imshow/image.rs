use std::borrow::{Borrow, Cow};
use std::time::Instant;

use glium::{
    backend::Facade,
    texture::{ClientFormat, RawImage2d},
    Texture2d,
};
use imgui::ImTexture;
use ndarray::Array2;

use super::lut::ColorLUT;
use super::{Error, Textures};
use lims;

fn make_raw_image<'a>(
    image: &Array2<f32>,
    vmin: f32,
    vmax: f32,
    lut: &ColorLUT,
) -> Result<RawImage2d<'a, u8>, Error> {
    let (m, n) = image.dim();
    let mut data = Vec::with_capacity(3 * n * m);

    if !vmin.is_nan() && !vmax.is_nan() {
        for val in image.iter() {
            // Make data
            let [r, g, b] = lut.color_at_bounds(*val, vmin, vmax);
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

pub struct Image<I> {
    vmin: f32,
    vmax: f32,
    tex_size: (f32, f32),
    created_on: Option<Instant>,
    data: Option<I>,
}

impl<I> Default for Image<I> {
    fn default() -> Self {
        use std::f32;
        Self {
            vmin: f32::NAN,
            vmax: f32::NAN,
            tex_size: (0.0, 0.0),
            created_on: None,
            data: None,
        }
    }
}

impl<I> Image<I>
where
    I: Borrow<Array2<f32>>,
{
    pub fn new<F>(
        image: I,
        created_on: Instant,
        ctx: &F,
        texture_id: ImTexture,
        textures: &mut Textures,
        lut: &ColorLUT,
    ) -> Result<Self, Error>
    where
        F: Facade,
    {
        let (vmin, vmax, tex_size) = {
            let image = image.borrow();
            let vmin = lims::get_vmin(image)?;
            let vmax = lims::get_vmax(image)?;

            let raw = make_raw_image(image, vmin, vmax, lut)?;
            let gl_texture = Texture2d::new(ctx, raw)?;
            let tex_size = gl_texture.dimensions();
            let tex_size = (tex_size.0 as f32, tex_size.1 as f32);
            textures.replace(texture_id, gl_texture);

            (vmin, vmax, tex_size)
        };

        Ok(Image {
            vmin,
            vmax,
            tex_size,
            created_on: Some(created_on),
            data: Some(image),
        })
    }

    pub fn update_texture<F>(
        &self,
        ctx: &F,
        texture_id: ImTexture,
        textures: &mut Textures,
        lut: &ColorLUT,
    ) -> Result<(), Error>
    where
        F: Facade,
    {
        if let Some(data) = &self.data {
            let raw = make_raw_image(data.borrow(), self.vmin, self.vmax, lut)?;
            let gl_texture = Texture2d::new(ctx, raw)?;
            textures.replace(texture_id, gl_texture);
        }
        Ok(())
    }

    pub fn get(&self, index: (usize, usize)) -> Option<f32> {
        self.data
            .as_ref()
            .and_then(|data| data.borrow().get(index).cloned())
    }

    pub fn dim(&self) -> (usize, usize) {
        self.data.as_ref().expect("Image is cached").borrow().dim()
    }

    pub fn inner(&self) -> &Array2<f32> {
        self.data.as_ref().expect("Image is cached").borrow()
    }

    pub fn vmin(&self) -> f32 {
        self.vmin
    }
    pub fn vmax(&self) -> f32 {
        self.vmax
    }
    pub fn tex_size(&self) -> (f32, f32) {
        self.tex_size
    }
    pub fn created_on(&self) -> Option<Instant> {
        self.created_on
    }
}
