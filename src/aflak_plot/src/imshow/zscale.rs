/*
This file is based on the part of STScI numdisplay package:
    https://github.com/spacetelescope/stsci.numdisplay/blob/b5062ec20673066b98d00e31fe165bb028db5181/lib/stsci/numdisplay/zscale.py

under the following license:

Copyright (C) 2005 Association of Universities for Research in Astronomy (AURA)

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

    1. Redistributions of source code must retain the above copyright
      notice, this list of conditions and the following disclaimer.

    2. Redistributions in binary form must reproduce the above
      copyright notice, this list of conditions and the following
      disclaimer in the documentation and/or other materials provided
      with the distribution.

    3. The name of AURA and its representatives may not be used to
      endorse or promote products derived from this software without
      specific prior written permission.

THIS SOFTWARE IS PROVIDED BY AURA ``AS IS'' AND ANY EXPRESS OR IMPLIED
WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF
MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL AURA BE LIABLE FOR ANY DIRECT, INDIRECT,
INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS
OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND
ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR
TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH
DAMAGE.
*/

use super::image::{coerce_to_array_view2, Image};
use super::Error;
use ndarray::{ArrayD, Axis};
use std::borrow::Borrow;

impl<I> Image<I>
where
    I: Borrow<ArrayD<f32>>,
{
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
            let image_val = self.data();
            let image_val = image_val.expect("Image is cached");
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
