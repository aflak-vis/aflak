use ndarray::{ArrayBase, Data, Ix2, Ix3};

#[derive(Copy, Clone, Debug)]
pub struct Bin {
    pub start: f32,
    pub end: f32,
    pub count: usize,
}

pub fn histogram<S>(data: &ArrayBase<S, Ix2>, min: f32, max: f32) -> Vec<Bin>
where
    S: Data<Elem = f32>,
{
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
        let i = (*val - min) / (max - min) * HISTOGRAM_BIN_COUNT as f32;
        let mut i = i as usize;
        if i >= HISTOGRAM_BIN_COUNT {
            i = HISTOGRAM_BIN_COUNT - 1;
        }
        bins[i].count += 1;
    }
    bins
}

pub fn histogram_color<S>(data: &ArrayBase<S, Ix3>, min: f32, max: f32) -> Vec<Bin>
where
    S: Data<Elem = f32>,
{
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
        let i = (*val - min) / (max - min) * HISTOGRAM_BIN_COUNT as f32;
        let mut i = i as usize;
        if i >= HISTOGRAM_BIN_COUNT {
            i = HISTOGRAM_BIN_COUNT - 1;
        }
        bins[i].count += 1;
    }
    bins
}
