//! A node editor library built on top of `aflak_cake` and `imgui`.
//!
//! For development you will want to check the
//! [NodeEditor](struct.NodeEditor.html) struct.
extern crate aflak_cake as cake;
#[macro_use]
extern crate imgui;
extern crate ron;

extern crate serde;
#[macro_use]
extern crate serde_derive;

mod compute;
mod constant_editor;
mod editor;
mod export;
mod id_stack;
mod node_state;
mod scrolling;
mod vec2;

pub use compute::ComputationState;
pub use constant_editor::ConstantEditor;
pub use editor::NodeEditor;
