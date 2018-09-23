use std::io;
use std::path::Path;

use fitrs::{Fits, Hdu};

use super::IOValue;

impl IOValue {
    pub fn save<P: AsRef<Path>>(&self, path: P) -> io::Result<Fits> {
        Fits::create(
            path,
            match self {
                IOValue::Image1d(arr) => Hdu::new(
                    arr.shape(),
                    arr.as_slice()
                        .expect("Could not get slice out of array")
                        .to_owned(),
                ),
                IOValue::Image2d(arr) => Hdu::new(
                    arr.shape(),
                    arr.as_slice()
                        .expect("Could not get slice out of array")
                        .to_owned(),
                ),
                _ => unimplemented!("Can only save Image1d and Image2d"),
            },
        )
    }
}
