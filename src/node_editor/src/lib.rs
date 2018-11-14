extern crate aflak_cake as cake;
#[macro_use]
extern crate imgui;
extern crate rayon;
extern crate ron;

extern crate serde;
#[macro_use]
extern crate serde_derive;

mod compute;
mod constant_editor;
mod editor;
mod export;
mod id_stack;
mod node_editable;
mod node_state;
mod scrolling;
mod vec2;

pub use compute::{ComputationState, ComputeResult};
pub use constant_editor::ConstantEditor;
pub use editor::{MainEditor, NodeEditor};
