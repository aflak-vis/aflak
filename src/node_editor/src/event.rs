use std::error;

use cake::{InputSlot, NodeId, Output, Transform, TransformIdx};

use imgui::ImString;

pub enum RenderEvent<T: 'static, E: 'static> {
    Connect(Output, InputSlot),
    AddTransform(&'static Transform<T, E>),
    CreateOutput,
    AddConstant(&'static str),
    SetConstant(TransformIdx, Box<T>),
    WriteDefaultInput {
        t_idx: TransformIdx,
        input_index: usize,
        val: Box<T>,
    },
    RemoveNode(NodeId),
    Error(Box<error::Error>),
    Success(ImString),
    Import,
}
