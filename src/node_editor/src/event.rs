use std::fmt;

use cake::{InputSlot, NodeId, Output, Transform, TransformIdx};

pub enum RenderEvent<T: 'static, E: 'static> {
    Connect(Output, InputSlot),
    AddTransform(&'static Transform<'static, T, E>),
    CreateOutput,
    AddConstant(&'static str),
    SetConstant(TransformIdx, Box<T>),
    WriteDefaultInput {
        t_idx: TransformIdx,
        input_index: usize,
        val: Box<T>,
    },
    RemoveNode(NodeId),
    Import,
    Export,
    AddNewMacro,
}

impl<T, E> fmt::Debug for RenderEvent<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::RenderEvent::*;
        match self {
            Connect(o, i) => write!(f, "Connect({:?}, {:?})", o, i),
            AddTransform(_) => write!(f, "AddTransform(_)"),
            CreateOutput => write!(f, "CreateOutput"),
            AddConstant(name) => write!(f, "AddConstant({:?})", name),
            SetConstant(t_idx, _) => write!(f, "SetConstant({:?}, _)", t_idx),
            WriteDefaultInput {
                t_idx, input_index, ..
            } => write!(
                f,
                "WriteDefaultInput {{ t_idx: {:?}, input_index: {:?}, .. }}",
                t_idx, input_index
            ),
            RemoveNode(node_id) => write!(f, "RemoveNode({:?})", node_id),
            Import => write!(f, "Import"),
            Export => write!(f, "Export"),
            AddNewMacro => write!(f, "AddNewMacro"),
        }
    }
}
