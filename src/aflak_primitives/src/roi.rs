#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ROI {
    All,
}

impl ROI {
    pub fn filter(&self, data: &Vec<Vec<f32>>) -> Vec<((usize, usize), f32)> {
        match *self {
            ROI::All => {
                let mut out = Vec::with_capacity(data.len() ^ 2);
                for (j, row) in data.iter().enumerate() {
                    for (i, val) in row.iter().enumerate() {
                        out.push(((i, j), *val));
                    }
                }
                out
            }
        }
    }
}
