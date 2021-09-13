use std::fmt;

use crate::cake::{macros, InputSlot, NodeId, Output, Transform, TransformIdx};

pub enum RenderEvent<T: 'static, E: 'static> {
    Connect(Output, InputSlot),
    Disconnect(Output, InputSlot),
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
    AddMacro(macros::MacroHandle<'static, T, E>),
    EditNode(NodeId),
    ChangeOutputName(NodeId, String),
}

impl<T, E> fmt::Debug for RenderEvent<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::RenderEvent::*;
        match self {
            Connect(o, i) => write!(f, "Connect({:?}, {:?})", o, i),
            Disconnect(o, i) => write!(f, "Disconnect({:?}, {:?})", o, i),
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
            AddMacro(handle) => write!(f, "AddMacro(id={}, name={:?})", handle.id(), handle.name()),
            EditNode(node_id) => write!(f, "EditNode({:?})", node_id),
            ChangeOutputName(node_id, name) => {
                write!(f, "ChangeOutputName(id={:?}, name={})", node_id, name)
            }
        }
    }
}

pub trait ApplyRenderEvent<T, E> {
    fn apply_event(&mut self, ev: RenderEvent<T, E>) {
        use crate::event::RenderEvent::*;
        match ev {
            Connect(output, input_slot) => self.connect(output, input_slot),
            Disconnect(output, input_slot) => self.disconnect(output, input_slot),
            AddTransform(t) => self.add_transform(t),
            CreateOutput => self.create_output(),
            AddConstant(constant_type) => self.add_constant(constant_type),
            SetConstant(t_idx, val) => self.set_constant(t_idx, val),
            WriteDefaultInput {
                t_idx,
                input_index,
                val,
            } => self.write_default_input(t_idx, input_index, val),
            RemoveNode(node_id) => self.remove_node(node_id),
            Import => self.import(),
            Export => self.export(),
            AddNewMacro => self.add_new_macro(),
            AddMacro(handle) => self.add_macro(handle),
            EditNode(node_id) => self.edit_node(node_id),
            ChangeOutputName(node_id, name) => self.change_output_name(node_id, name),
        }
    }

    fn connect(&mut self, output: Output, input_slot: InputSlot);
    fn disconnect(&mut self, output: Output, input_slot: InputSlot);
    fn add_transform(&mut self, t: &'static Transform<'static, T, E>);
    fn create_output(&mut self);
    fn add_constant(&mut self, constant_type: &'static str);
    fn set_constant(&mut self, t_idx: TransformIdx, c: Box<T>);
    fn write_default_input(&mut self, t_idx: TransformIdx, input_index: usize, val: Box<T>);
    fn remove_node(&mut self, node_id: NodeId);
    fn import(&mut self);
    fn export(&mut self);
    fn add_new_macro(&mut self);
    fn add_macro(&mut self, handle: macros::MacroHandle<'static, T, E>);
    fn edit_node(&mut self, node: NodeId);
    fn change_output_name(&mut self, node_id: NodeId, name: String);
}
