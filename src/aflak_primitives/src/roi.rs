use ndarray::ArrayView2;

/// A region of interest in a 2D image.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ROI {
    /// The whole image is a region of interest.
    All,
    /// The list of pixels selected by this region of interest.
    PixelList(Vec<(usize, usize)>),
}

impl ROI {
    /// Get the value of each point of the 2D image in the region of interest,
    /// along with the original coordinate of each selected pixel.
    pub fn filter(&self, data: ArrayView2<f32>) -> Vec<((usize, usize), f32)> {
        match *self {
            ROI::All => {
                let mut out = Vec::with_capacity(data.len());
                let size = data.dim();
                for j in 0..size.1 {
                    for i in 0..size.0 {
                        out.push(((i, j), *data.get([i, j]).unwrap()));
                    }
                }
                out
            }
            ROI::PixelList(ref pixels) => {
                let mut out = Vec::with_capacity(pixels.len());
                for &(i, j) in pixels {
                    if let Some(val) = data.get([i, j]) {
                        out.push(((i, j), *val));
                    }
                }
                out
            }
        }
    }

    pub fn filterx(&self, data: ArrayView2<f32>) -> Vec<((usize, usize), f32)> {
        match *self {
            ROI::All => {
                let mut out = Vec::with_capacity(data.len());
                let size = data.dim();
                for j in 0..size.1 {
                    for i in 0..size.0 {
                        out.push(((i, j), *data.get([i, j]).unwrap()));
                    }
                }
                out
            }
            ROI::PixelList(ref pixels) => {
                let mut out = Vec::with_capacity(pixels.len());
                for &(i, j) in pixels {
                    if let Some(val) = data.get([73 - j, i]) {
                        out.push(((i, j), *val));
                    }
                }
                out
            }
        }
    }

    pub fn datalen(&self) -> usize {
        match *self {
            ROI::All => 0,
            ROI::PixelList(ref pixels) => pixels.len(),
        }
    }
}
