//! # aflak - Computational mAKE
//!
//! A crate to manage a graph of interdependent functions.
extern crate serde;
#[macro_use]
extern crate serde_derive;

extern crate variant_name;

mod transform;
mod dst;
mod serial;

pub use transform::*;
pub use dst::*;
