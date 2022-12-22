//! Plotting library for aflak.
//!
//! Please see the examples in the repository of this crate to get an idea of
//! how it is used.
//!
//! Basically, this crate defines and implements two traits on imgui's `Ui`
//! objet. These are [UiImage1d](plot/trait.UiImage1d.html) and
//! [UiImage2d](imshow/trait.UiImage2d.html).
extern crate aflak_cake as cake;
extern crate aflak_primitives as primitives;
extern crate glium;
extern crate imgui;
extern crate imgui_glium_renderer;
extern crate node_editor;
#[macro_use]
extern crate ndarray;

extern crate implot;
extern crate meval;

pub mod imshow;
pub mod persistence_diagram;
pub mod plot;
pub mod plot_colormap;
pub mod scatter_lineplot;
pub mod three;

mod err;
pub mod interactions;
mod lims;
mod ticks;
mod units;
mod util;

pub use crate::err::Error;
pub use crate::interactions::{Interaction, InteractionId, InteractionIterMut, Value, ValueIter};
pub use crate::units::AxisTransform;
