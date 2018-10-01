extern crate glium;
#[macro_use]
extern crate aflak_imgui as imgui;
extern crate aflak_imgui_glium_renderer as imgui_glium_renderer;
extern crate ndarray;

pub mod imshow;
pub mod plot;

mod err;
mod interactions;
mod lims;
mod ticks;
mod util;

pub use err::Error;
pub use interactions::{InteractionId, InteractionIterMut, Value, ValueIter};
