use std::borrow::{Borrow, Cow};
use std::rc::Rc;
use std::time::Instant;

use glium::{
    backend::Facade,
    texture::{ClientFormat, RawImage2d},
    uniforms::{MagnifySamplerFilter, SamplerBehavior},
    Texture2d,
};
use imgui::TextureId;
use imgui_glium_renderer::Texture;
use ndarray::{ArrayBase, ArrayD, ArrayView2, ArrayView3, Axis, Data, Dimension, Ix2, Ix3};

use super::hist;
use super::lut::ColorLUT;
use super::{Error, Textures};
use crate::lims;

fn make_raw_image<S>(
    image: &ArrayBase<S, Ix2>,
    vmin: f32,
    vmax: f32,
    lut: &ColorLUT,
) -> Result<RawImage2d<'static, u8>, Error>
where
    S: Data<Elem = f32>,
{
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

fn make_raw_image_rgb<S>(
    image: &ArrayBase<S, Ix3>,
    vmin: f32,
    vmax: f32,
    lut: &ColorLUT,
) -> Result<RawImage2d<'static, u8>, Error>
where
    S: Data<Elem = f32>,
{
    let (_c, m, n) = image.dim();
    let lims = lut.lims();
    let mut data = Vec::with_capacity(3 * n * m);
    if !vmin.is_nan() && !vmax.is_nan() {
        for (_, slice) in image.axis_iter(Axis(1)).enumerate() {
            for (_, channel) in slice.axis_iter(Axis(1)).enumerate() {
                let lim_min = lims.0 * vmax;
                let lim_max = lims.2 * vmax;
                let r = (channel[0] - lim_min) / (lim_max - lim_min) * 255.0;
                let g = (channel[1] - lim_min) / (lim_max - lim_min) * 255.0;
                let b = (channel[2] - lim_min) / (lim_max - lim_min) * 255.0;
                data.push(r as u8);
                data.push(g as u8);
                data.push(b as u8);
            }
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
    vmin_rgb: [f32; 3],
    vmax: f32,
    vmax_rgb: [f32; 3],
    vmed: f32,
    vmed_rgb: [f32; 3],
    vmad: f32,
    vmad_rgb: [f32; 3],
    tex_size: (f32, f32),
    created_on: Option<Instant>,
    data: Option<I>,
    hist: Vec<hist::Bin>,
    hist_color: Vec<[hist::Bin; 3]>,
}

impl<I> Default for Image<I> {
    fn default() -> Self {
        use std::f32;
        Self {
            vmin: f32::NAN,
            vmin_rgb: [f32::NAN; 3],
            vmax: f32::NAN,
            vmax_rgb: [f32::NAN; 3],
            vmed: f32::NAN,
            vmed_rgb: [f32::NAN; 3],
            vmad: f32::NAN,
            vmad_rgb: [f32::NAN; 3],
            tex_size: (0.0, 0.0),
            created_on: None,
            data: None,
            hist: vec![],
            hist_color: vec![],
        }
    }
}

pub fn coerce_to_array_view2<I, A>(image: &I) -> ArrayView2<'_, A>
where
    I: Borrow<ArrayD<A>>,
{
    let image = image.borrow();
    image.slice(s![.., ..])
}

fn coerce_to_array_view3<I, A>(image: &I) -> ArrayView3<'_, A>
where
    I: Borrow<ArrayD<A>>,
{
    let image = image.borrow();
    image.slice(s![.., .., ..])
}

impl<I> Image<I>
where
    I: Borrow<ArrayD<f32>>,
{
    pub fn new<F>(
        image: I,
        created_on: Instant,
        ctx: &F,
        texture_id: TextureId,
        textures: &mut Textures,
        lut: &ColorLUT,
    ) -> Result<Image<I>, Error>
    where
        F: Facade,
    {
        let (vmin, vmax, vmed, vmad, tex_size, hist) = {
            let image = coerce_to_array_view2(&image);
            let vmin = lims::get_vmin(&image)?;
            let vmax = lims::get_vmax(&image)?;
            let vmed = lims::get_vmed_normalized(&image)?;
            let vmad = lims::get_vmad_normalized(&image)?;
            let raw = make_raw_image(&image, vmin, vmax, lut)?;
            let gl_texture = Texture2d::new(ctx, raw)?;
            let tex_size = gl_texture.dimensions();
            let tex_size = (tex_size.0 as f32, tex_size.1 as f32);
            textures.replace(
                texture_id,
                Texture {
                    texture: Rc::new(gl_texture),
                    sampler: SamplerBehavior {
                        magnify_filter: MagnifySamplerFilter::Nearest,
                        ..Default::default()
                    },
                },
            );

            let hist = hist::histogram(&image, vmin, vmax);
            (vmin, vmax, vmed, vmad, tex_size, hist)
        };

        Ok(Image {
            vmin,
            vmin_rgb: [f32::NAN; 3],
            vmax,
            vmax_rgb: [f32::NAN; 3],
            vmed,
            vmed_rgb: [f32::NAN; 3],
            vmad,
            vmad_rgb: [f32::NAN; 3],
            tex_size,
            created_on: Some(created_on),
            data: Some(image),
            hist,
            hist_color: vec![],
        })
    }

    pub fn color_new<F>(
        image: I,
        created_on: Instant,
        ctx: &F,
        texture_id: TextureId,
        textures: &mut Textures,
        lut: &[ColorLUT; 3],
    ) -> Result<Image<I>, Error>
    where
        F: Facade,
    {
        let (vmin, vmin_rgb, vmax, vmax_rgb, vmed, vmed_rgb, vmad, vmad_rgb, tex_size, hist_color) = {
            let image = coerce_to_array_view3(&image);
            let mut vmin_rgb = [f32::NAN; 3];
            let mut vmax_rgb = [f32::NAN; 3];
            let mut vmed_rgb = [f32::NAN; 3];
            let mut vmad_rgb = [f32::NAN; 3];
            for (c, channels) in image.axis_iter(Axis(0)).enumerate() {
                vmin_rgb[c] = lims::get_vmin(&channels)?;
                vmax_rgb[c] = lims::get_vmax(&channels)?;
                vmed_rgb[c] = lims::get_vmed_normalized(&channels)?;
                vmad_rgb[c] = lims::get_vmad_normalized(&channels)?;
            }
            let vmin = lims::get_vmin(&image)?;
            let vmax = lims::get_vmax(&image)?;
            let vmed = lims::get_vmed_normalized(&image)?;
            let vmad = lims::get_vmad_normalized(&image)?;
            let raw = make_raw_image_rgb(&image, vmin_rgb, vmax_rgb, lut)?;
            let gl_texture = Texture2d::new(ctx, raw)?;
            let tex_size = gl_texture.dimensions();
            let tex_size = (tex_size.0 as f32, tex_size.1 as f32);
            textures.replace(
                texture_id,
                Texture {
                    texture: Rc::new(gl_texture),
                    sampler: SamplerBehavior {
                        magnify_filter: MagnifySamplerFilter::Nearest,
                        ..Default::default()
                    },
                },
            );

            let hist = hist::histogram_color(&image, 0.0, 65535.0);
            (
                vmin, vmin_rgb, vmax, vmax_rgb, vmed, vmed_rgb, vmad, vmad_rgb, tex_size, hist,
            )
        };

        Ok(Image {
            vmin,
            vmin_rgb,
            vmax,
            vmax_rgb,
            vmed,
            vmed_rgb,
            vmad,
            vmad_rgb,
            tex_size,
            created_on: Some(created_on),
            data: Some(image),
            hist: vec![],
            hist_color,
        })
    }

    pub fn update_texture<F>(
        &self,
        ctx: &F,
        texture_id: TextureId,
        textures: &mut Textures,
        lut: &ColorLUT,
    ) -> Result<(), Error>
    where
        F: Facade,
    {
        if let Some(data) = &self.data {
            let image = coerce_to_array_view2(data);
            let raw = make_raw_image(&image, self.vmin, self.vmax, lut)?;
            let gl_texture = Texture2d::new(ctx, raw)?;
            textures.replace(
                texture_id,
                Texture {
                    texture: Rc::new(gl_texture),
                    sampler: SamplerBehavior {
                        magnify_filter: MagnifySamplerFilter::Nearest,
                        ..Default::default()
                    },
                },
            );
        }
        Ok(())
    }

    pub fn update_texture_color<F>(
        &self,
        ctx: &F,
        texture_id: TextureId,
        textures: &mut Textures,
        lut: &[ColorLUT; 3],
    ) -> Result<(), Error>
    where
        F: Facade,
    {
        if let Some(data) = &self.data {
            let image = coerce_to_array_view3(data);
            let raw = make_raw_image_rgb(&image, self.vmin_rgb, self.vmax_rgb, lut)?;
            let gl_texture = Texture2d::new(ctx, raw)?;
            textures.replace(
                texture_id,
                Texture {
                    texture: Rc::new(gl_texture),
                    sampler: SamplerBehavior {
                        magnify_filter: MagnifySamplerFilter::Nearest,
                        ..Default::default()
                    },
                },
            );
        }
        Ok(())
    }

    pub fn get(&self, index: [usize; 2]) -> Option<f32> {
        self.data
            .as_ref()
            .and_then(|data| data.borrow().get(index))
            .cloned()
    }

    pub fn get_color(&self, index: [usize; 3]) -> Option<f32> {
        self.data
            .as_ref()
            .and_then(|data| data.borrow().get(index))
            .cloned()
    }

    pub fn dim(&self) -> (usize, usize) {
        let dim = self.data.as_ref().expect("Image is cached").borrow().dim();
        let dim_view = dim.as_array_view();
        (dim_view[0], dim_view[1])
    }

    pub fn ndim(&self) -> usize {
        let ndim = self.data.as_ref().expect("Image is cached").borrow().ndim();
        ndim
    }

    pub fn hist(&self) -> &[hist::Bin] {
        &self.hist
    }

    pub fn hist_color(&self) -> &[[hist::Bin; 3]] {
        &self.hist_color
    }

    pub fn vmin(&self) -> f32 {
        self.vmin
    }
    pub fn vmin_rgb(&self) -> [f32; 3] {
        self.vmin_rgb
    }
    pub fn vmax(&self) -> f32 {
        self.vmax
    }
    pub fn vmax_rgb(&self) -> [f32; 3] {
        self.vmax_rgb
    }
    pub fn vmed(&self) -> f32 {
        self.vmed
    }
    pub fn vmed_rgb(&self) -> [f32; 3] {
        self.vmed_rgb
    }
    pub fn vmad(&self) -> f32 {
        self.vmad
    }
    pub fn vmad_rgb(&self) -> [f32; 3] {
        self.vmad_rgb
    }
    pub fn tex_size(&self) -> (f32, f32) {
        self.tex_size
    }
    pub fn created_on(&self) -> Option<Instant> {
        self.created_on
    }
    pub fn data(&self) -> Option<&I> {
        self.data.as_ref()
    }
}
