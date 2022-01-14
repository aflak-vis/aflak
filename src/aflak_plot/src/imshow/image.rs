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
    vmax: f32,
    vmed: f32,
    vmad: f32,
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
            vmax: f32::NAN,
            vmed: f32::NAN,
            vmad: f32::NAN,
            tex_size: (0.0, 0.0),
            created_on: None,
            data: None,
            hist: vec![],
            hist_color: vec![],
        }
    }
}

fn coerce_to_array_view2<I, A>(image: &I) -> ArrayView2<'_, A>
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
            let vmed = lims::get_vmed_from_normalized_image(&image)?;
            let vmad = lims::get_vmad_from_normalized_image(&image)?;
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
            vmax,
            vmed,
            vmad,
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
        lut: &ColorLUT,
    ) -> Result<Image<I>, Error>
    where
        F: Facade,
    {
        let (vmin, vmax, vmed, vmad, tex_size, hist_color) = {
            let image = coerce_to_array_view3(&image);
            let vmin = lims::get_vmin(&image)?;
            let vmax = lims::get_vmax(&image)?;
            let vmed = lims::get_vmed_from_normalized_image(&image)?;
            let vmad = lims::get_vmad_from_normalized_image(&image)?;
            let raw = make_raw_image_rgb(&image, vmin, vmax, lut)?;
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
            (vmin, vmax, vmed, vmad, tex_size, hist)
        };

        Ok(Image {
            vmin,
            vmax,
            vmed,
            vmad,
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
        lut: &ColorLUT,
    ) -> Result<(), Error>
    where
        F: Facade,
    {
        if let Some(data) = &self.data {
            let image = coerce_to_array_view3(data);
            let raw = make_raw_image_rgb(&image, self.vmin, self.vmax, lut)?;
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
    pub fn vmax(&self) -> f32 {
        self.vmax
    }
    pub fn vmed(&self) -> f32 {
        self.vmed
    }
    pub fn vmad(&self) -> f32 {
        self.vmad
    }
    pub fn tex_size(&self) -> (f32, f32) {
        self.tex_size
    }
    pub fn created_on(&self) -> Option<Instant> {
        self.created_on
    }
    pub fn zscale(&self, nsamples: i32, contrast: f32) -> (Option<f32>, Option<f32>) {
        const MIN_NPIXELS: i32 = 5;
        const MAX_REJECT: f32 = 0.5;
        const KREJ: f32 = 2.5;
        const MAX_ITERATIONS: i32 = 5;
        let mut sample = self.zsc_sample(nsamples).unwrap();
        sample.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let npix = sample.len();
        if let (Some(&zmin), Some(&zmax)) = (sample.first(), sample.last()) {
            let center_pixel = (npix - 1) / 2;
            let median;
            if npix % 2 == 1 {
                median = sample[center_pixel];
            } else {
                median = 0.5 * (sample[center_pixel] + sample[center_pixel + 1]);
            }
            let minpix = MIN_NPIXELS.max((npix as f32 * MAX_REJECT) as i32);
            let ngrow = 1.max((npix as f32 * 0.01) as i32);
            let (ngoodpix, _zstart, mut zslope) = self
                .zsc_fit_line(
                    sample,
                    npix as i32,
                    KREJ,
                    ngrow,
                    MAX_ITERATIONS,
                    MIN_NPIXELS,
                    MAX_REJECT,
                )
                .unwrap();
            let z1;
            let z2;
            if ngoodpix < minpix {
                z1 = zmin;
                z2 = zmax;
            } else {
                if contrast > 0.0 {
                    zslope = zslope / contrast;
                }
                z1 = zmin.max(median - (center_pixel - 1) as f32 * zslope);
                z2 = zmax.min(median + (npix - center_pixel) as f32 * zslope);
            }
            (Some(z1), Some(z2))
        } else {
            (None, None)
        }
    }
    fn zsc_sample(&self, maxpix: i32) -> Result<Vec<f32>, Error> {
        let ndim = self.ndim();
        if ndim == 2 {
            let dim = self.dim();
            let nc = dim.0;
            let nl = dim.1;
            let image_val = self.data.as_ref().expect("Image is cached");
            let image_val = coerce_to_array_view2(image_val).to_owned();

            let stride = 1.max((((nc - 1) * (nl - 1)) as f32 / maxpix as f32).sqrt() as usize);
            let mut v = Vec::new();
            for line in image_val.axis_iter(Axis(0)).step_by(stride) {
                for &data in line.iter().step_by(stride) {
                    if !data.is_nan() {
                        v.push(data);
                    }
                }
            }
            v.truncate(maxpix as usize);
            Ok(v)
        } else {
            Err(Error::Msg("zsc sample failed."))
        }
    }
    fn zsc_fit_line(
        &self,
        samples: Vec<f32>,
        npix: i32,
        krej: f32,
        ngrow: i32,
        maxiter: i32,
        min_npixels: i32,
        max_reject: f32,
    ) -> Result<(i32, f32, f32), Error> {
        let xscale = 2.0 / (npix - 1) as f32;
        let xnorm: Vec<f32> = (0..npix)
            .into_iter()
            .map(|v| v as f32 * xscale - 1.0)
            .collect();
        let mut ngoodpix = npix;
        let minpix = min_npixels.max((npix as f32 * max_reject) as i32);
        let last_ngoodpix = npix + 1;
        let mut badpix = vec![0; npix as usize];
        const GOOD_PIXEL: i32 = 0;
        let mut intercept = 0.0;
        let mut slope = 0.0;
        for _niter in 0..maxiter {
            if ngoodpix >= last_ngoodpix || ngoodpix < minpix {
                break;
            }
            let mut badpix_iter = badpix.iter();
            let mut xnorm_cloned = xnorm.clone();
            let mut samples_cloned = samples.clone();
            xnorm_cloned.retain(|_| *badpix_iter.next().unwrap() == GOOD_PIXEL);
            let mut badpix_iter = badpix.iter();
            samples_cloned.retain(|_| *badpix_iter.next().unwrap() == GOOD_PIXEL);
            let sum = xnorm_cloned.len();
            let sumx: f32 = xnorm_cloned.iter().sum();
            let sumxx: f32 = xnorm_cloned.iter().map(|x| x * x).sum();
            let sumxy: f32 = xnorm_cloned
                .iter()
                .zip(samples_cloned.iter())
                .map(|(x, y)| x * y)
                .sum();
            let sumy: f32 = samples_cloned.iter().sum();
            let delta = sum as f32 * sumxx - sumx * sumx;
            intercept = (sumxx * sumy - sumx * sumxy) / delta;
            slope = (sum as f32 * sumxy - sumx * sumy) / delta;

            let fitted: Vec<f32> = xnorm.iter().map(|x| x * slope + intercept).collect();
            let flat: Vec<f32> = samples
                .iter()
                .zip(fitted.iter())
                .map(|(x, y)| x - y)
                .collect();
            let (ng, mean, sigma) = self
                .zsc_compute_sigma(flat.clone(), badpix.clone())
                .unwrap();
            ngoodpix = ng;
            if let (Some(_mean), Some(sigma)) = (mean, sigma) {
                let threshold = sigma * krej;
                let lcut = -threshold;
                let hcut = threshold;
                let next_badpix = badpix
                    .iter()
                    .zip(flat.iter())
                    .map(|(_, f)| if *f < lcut || *f > hcut { 1 } else { 0 })
                    .collect();
                let kernel = vec![1, ngrow];
                badpix = convolve(next_badpix, kernel);
                ngoodpix = badpix
                    .iter()
                    .filter(|&&x| x == GOOD_PIXEL)
                    .cloned()
                    .collect::<Vec<i32>>()
                    .len() as i32;
                fn convolve(target: Vec<i32>, filter: Vec<i32>) -> Vec<i32> {
                    let mut ret = Vec::new();
                    for head in 0..target.len() {
                        let mut data = 0;
                        for m in 0..filter.len() {
                            if head < m {
                                break;
                            }
                            data += target[head - m] * filter[m];
                        }
                        ret.push(data);
                    }
                    ret
                }
            }
        }
        let zstart = intercept - slope;
        let zslope = slope * xscale;
        Ok((ngoodpix, zstart, zslope))
    }

    fn zsc_compute_sigma(
        &self,
        mut flat: Vec<f32>,
        badpix: Vec<i32>,
    ) -> Result<(i32, Option<f32>, Option<f32>), Error> {
        const GOOD_PIXEL: i32 = 0;
        let mut badpix_iter = badpix.iter();
        flat.retain(|_| *badpix_iter.next().unwrap() == GOOD_PIXEL);
        let sumz = flat.iter().sum();
        let sumsq: f32 = flat.iter().map(|x| x * x).sum();
        let mut badpix_cloned = badpix.clone();
        badpix_cloned.retain(|v| *v == GOOD_PIXEL);
        let ngoodpix = badpix_cloned.len() as i32;
        let mean;
        let sigma;
        if ngoodpix == 0 {
            mean = None;
            sigma = None;
        } else if ngoodpix == 1 {
            mean = Some(sumz);
            sigma = None;
        } else {
            mean = Some(sumz / ngoodpix as f32);
            let temp: f32 =
                sumsq / (ngoodpix - 1) as f32 - sumz * sumz / (ngoodpix * (ngoodpix - 1)) as f32;
            if temp < 0.0 {
                sigma = Some(0.0);
            } else {
                sigma = Some(temp.sqrt());
            }
        }
        Ok((ngoodpix, mean, sigma))
    }
}
