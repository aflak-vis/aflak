use ndarray::ArrayView2;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ROI {
    All,
    PixelList(Vec<(usize, usize)>),
}

impl ROI {
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
}
