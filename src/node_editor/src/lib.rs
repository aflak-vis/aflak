extern crate aflak_cake as cake;
#[macro_use]
extern crate imgui;
extern crate rayon;

mod constant_editor;
mod compute;
mod editor;
mod id_stack;
mod node_state;
mod vec2;

pub use compute::ComputationState;
pub use editor::NodeEditor;
pub use constant_editor::ConstantEditor;
