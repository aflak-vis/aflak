use ndarray::Array2;

#[derive(Copy, Clone, Debug)]
pub struct Bin {
    pub start: f32,
    pub end: f32,
    pub count: usize,
}

pub fn histogram(data: &Array2<f32>, min: f32, max: f32) -> Vec<Bin> {
    const HISTOGRAM_BIN_COUNT: usize = 100;
    let mut bins = Vec::with_capacity(HISTOGRAM_BIN_COUNT);
    let size = (max - min) / HISTOGRAM_BIN_COUNT as f32;
    for i in 0..HISTOGRAM_BIN_COUNT {
        bins.push(Bin {
            start: min + i as f32 * size,
            end: min + (i + 1) as f32 * size,
            count: 0,
        });
    }

    for val in data.iter() {
        for bin in bins.iter_mut() {
            if bin.start <= *val && *val <= bin.end {
                bin.count += 1;
            }
        }
    }
    bins
}
