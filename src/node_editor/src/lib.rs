extern crate aflak_cake as cake;
#[macro_use]
extern crate imgui;
extern crate rayon;

mod compute;
mod constant_editor;
mod editor;
mod id_stack;
mod node_state;
mod vec2;

pub use compute::{ComputationState, ComputeResult};
pub use constant_editor::ConstantEditor;
pub use editor::NodeEditor;
