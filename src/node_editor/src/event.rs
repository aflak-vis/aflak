use std::{fmt, path::PathBuf};

use super::export::{ExportError, ImportError};
use crate::cake::{macros, InputSlot, NodeId, Output, OutputId, Transform, TransformIdx};
pub enum RenderEvent<T: 'static, E: 'static> {
    Connect(Output, InputSlot),
    Disconnect(Output, InputSlot),
    AddTransform(&'static Transform<'static, T, E>),
    AddOwnedTransform(
        Option<Transform<'static, T, E>>,
        Vec<Option<T>>,
        Vec<Option<Output>>,
        Vec<(Output, InputSlot)>,
    ),
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
    Undo,
    Redo,
}
#[derive(Clone)]
pub enum ProvenanceEvent<T: 'static, E: 'static> {
    Connect(Output, InputSlot, Result<(), cake::DSTError>),
    Disconnect(Output, InputSlot, Result<(), cake::DSTError>),
    AddTransform(&'static Transform<'static, T, E>, TransformIdx),
    AddOwnedTransform(
        Option<Transform<'static, T, E>>,
        TransformIdx,
        Vec<Option<T>>,
        Vec<Option<Output>>,
        Vec<(Output, InputSlot)>,
    ),
    CreateOutput(OutputId),
    AddConstant(&'static str, TransformIdx),
    SetConstant(TransformIdx, Option<T>, Box<T>, Result<(), ()>),
    WriteDefaultInput(TransformIdx, usize, Option<T>, Box<T>, Result<(), ()>),
    RemoveNode(
        NodeId,
        Option<Transform<'static, T, E>>,
        Option<String>,
        Vec<Option<T>>,
        Vec<Option<Output>>,
        Vec<(Output, InputSlot)>,
    ),
    Import(
        Option<PathBuf>,
        Option<cake::DST<'static, T, E>>,
        cake::DST<'static, T, E>,
        Result<(), ImportError>,
    ),
    Export(PathBuf, cake::DST<'static, T, E>, Result<(), ExportError>),
    AddNewMacro(TransformIdx),
    AddMacro(macros::MacroHandle<'static, T, E>, TransformIdx),
    EditNode(NodeId),
    ChangeOutputName(NodeId, String, String),
}

impl<T, E> From<ProvenanceEvent<T, E>> for RenderEvent<T, E> {
    fn from(p: ProvenanceEvent<T, E>) -> Self {
        match p {
            ProvenanceEvent::Connect(o, i, _) => RenderEvent::Connect(o, i),
            ProvenanceEvent::Disconnect(o, i, _) => RenderEvent::Disconnect(o, i),
            ProvenanceEvent::AddTransform(t, _) => RenderEvent::AddTransform(t),
            ProvenanceEvent::AddOwnedTransform(t, _, default_inputs, i_c, o_c) => {
                RenderEvent::AddOwnedTransform(t, default_inputs, i_c, o_c)
            }
            ProvenanceEvent::CreateOutput(_) => RenderEvent::CreateOutput,
            ProvenanceEvent::AddConstant(name, _) => RenderEvent::AddConstant(name),
            ProvenanceEvent::SetConstant(t_idx, _, val, _) => RenderEvent::SetConstant(t_idx, val),
            ProvenanceEvent::WriteDefaultInput(t_idx, input_index, _, val, _) => {
                RenderEvent::WriteDefaultInput {
                    t_idx,
                    input_index,
                    val,
                }
            }
            ProvenanceEvent::RemoveNode(n, _, _, _, _, _) => RenderEvent::RemoveNode(n),
            ProvenanceEvent::Import(_, _, _, _) => RenderEvent::Import,
            ProvenanceEvent::Export(_, _, _) => RenderEvent::Export,
            ProvenanceEvent::AddNewMacro(_) => RenderEvent::AddNewMacro,
            ProvenanceEvent::AddMacro(h, _) => RenderEvent::AddMacro(h),
            ProvenanceEvent::EditNode(n) => RenderEvent::EditNode(n),
            ProvenanceEvent::ChangeOutputName(n, _, after_name) => {
                RenderEvent::ChangeOutputName(n, after_name)
            }
        }
    }
}

impl<T: Clone, E> RenderEvent<T, E> {
    pub fn new(ev: &Self) -> Self {
        use self::RenderEvent::*;
        match ev {
            Connect(o, i) => Connect(*o, *i),
            Disconnect(o, i) => Disconnect(*o, *i),
            AddTransform(t) => AddTransform(t),
            AddOwnedTransform(t, d_i, i_c, o_c) => {
                AddOwnedTransform(t.clone(), d_i.clone(), i_c.clone(), o_c.clone())
            }
            CreateOutput => CreateOutput,
            AddConstant(name) => AddConstant(name),
            SetConstant(t_idx, v) => {
                let d = v.clone();
                SetConstant(*t_idx, d)
            }
            WriteDefaultInput {
                t_idx,
                input_index,
                val,
            } => {
                let val = val.clone();
                WriteDefaultInput {
                    t_idx: *t_idx,
                    input_index: *input_index,
                    val: val,
                }
            }
            RemoveNode(node_id) => RemoveNode(*node_id),
            Import => Import,
            Export => Export,
            AddNewMacro => AddNewMacro,
            AddMacro(handle) => AddMacro(handle.clone()),
            EditNode(node_id) => EditNode(*node_id),
            ChangeOutputName(node_id, name) => ChangeOutputName(*node_id, name.clone()),
            Undo => Undo,
            Redo => Redo,
        }
    }
}

impl<T, E> fmt::Debug for RenderEvent<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::RenderEvent::*;
        match self {
            Connect(o, i) => write!(f, "Connect({:?}, {:?})", o, i),
            Disconnect(o, i) => write!(f, "Disconnect({:?}, {:?})", o, i),
            AddTransform(_) => write!(f, "AddTransform(_)"),
            AddOwnedTransform(_, _, _, _) => write!(f, "AddSpecificTransform(_)"),
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
            Undo => write!(f, "Undo"),
            Redo => write!(f, "Redo"),
        }
    }
}

impl<T: fmt::Debug, E: fmt::Debug> fmt::Debug for ProvenanceEvent<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::ProvenanceEvent::*;
        match self {
            Connect(o, i, res) => write!(f, "Connect(({:?}, {:?}), {:?}),", o, i, res),
            Disconnect(o, i, res) => write!(f, "Disconnect(({:?}, {:?}), {:?})", o, i, res),
            AddTransform(t, t_idx) => write!(f, "AddTransform({:?}, {:?})", t_idx, t),
            AddOwnedTransform(t, t_idx, d_i, i_c, o_c) => {
                write!(
                    f,
                    "AddSpecificTransform({:?}, {:?}, {:?}, {:?}, {:?})",
                    t_idx, t, d_i, i_c, o_c
                )
            }
            CreateOutput(output_id) => write!(f, "CreateOutput({:?})", output_id),
            AddConstant(name, t_idx) => write!(f, "AddConstant({:?}, {:?})", name, t_idx),
            SetConstant(t_idx, before, after, res) => write!(
                f,
                "SetConstant({:?}, {:?} -> {:?}, {:?})",
                t_idx, before, after, res
            ),
            WriteDefaultInput(t_idx, input_index, before, after, res) => write!(
                f,
                "WriteDefaultInput(t_idx: {:?}, input_index: {:?}, {:?} -> {:?}, result: {:?})",
                t_idx, input_index, before, after, res
            ),
            RemoveNode(node_id, t, name, default_inputs, i_c, o_c) => {
                write!(
                    f,
                    "RemoveNode({:?}, transform: {:?}, name: {:?}, default_inputs: {:?}, inputside_connects: {:?}, outputside_connects: {:?})",
                    node_id, t, name, default_inputs, i_c, o_c
                )
            }
            Import(p, before_dst, after_dst, res) => {
                if let Some(before_dst) = before_dst {
                    write!(
                        f,
                        "Import from {:?}, DST(updated: {:?}) -> DST(updated: {:?}), Result: {:?}",
                        p,
                        before_dst.max_updated_on(),
                        after_dst.max_updated_on(),
                        res
                    )
                } else {
                    write!(
                        f,
                        "Import from {:?}, None -> DST(updated: {:?}), Result: {:?}",
                        p,
                        after_dst.max_updated_on(),
                        res
                    )
                }
            }
            Export(p, dst, res) => write!(
                f,
                "Export to {:?}, DST(updated: {:?}), Result: {:?}",
                p,
                dst.max_updated_on(),
                res
            ),
            AddNewMacro(t_idx) => write!(f, "AddNewMacro({:?})", t_idx),
            AddMacro(handle, t_idx) => write!(
                f,
                "AddMacro(id={}, name={:?}, t_idx={:?})",
                handle.id(),
                handle.name(),
                t_idx
            ),
            EditNode(node_id) => write!(f, "EditNode({:?})", node_id),
            ChangeOutputName(node_id, before_name, after_name) => {
                write!(
                    f,
                    "ChangeOutputName(id={:?}, name={} -> {})",
                    node_id, before_name, after_name
                )
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
            AddOwnedTransform(t, d_i, i_c, o_c) => self.add_owned_transform(t, d_i, i_c, o_c),
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
            Undo | Redo => {}
        }
    }

    fn connect(&mut self, output: Output, input_slot: InputSlot);
    fn disconnect(&mut self, output: Output, input_slot: InputSlot);
    fn add_transform(&mut self, t: &'static Transform<'static, T, E>);
    fn add_owned_transform(
        &mut self,
        t: Option<Transform<'static, T, E>>,
        d_i: Vec<Option<T>>,
        i_c: Vec<Option<Output>>,
        o_c: Vec<(Output, InputSlot)>,
    );
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
