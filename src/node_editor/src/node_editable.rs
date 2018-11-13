use std::collections::BTreeMap;
use std::error;
use std::io;

use std::ops::{Deref, DerefMut};

use cake::{Input, Macro, NodeId, Output, OutputId, DST};

use compute::ComputeResult;
use export::{ExportError, ImportError};
use node_state::{NodeState, NodeStates};
use scrolling::Scrolling;
use vec2::Vec2;

pub struct ImportSuccess<T> {
    pub object: T,
    pub node_states: NodeStates,
    pub scrolling: Scrolling,
}

pub struct ExportInput<'e> {
    node_states: Vec<(&'e NodeId, &'e NodeState)>,
    scrolling: Vec2,
}

pub struct DSTHandle<'a, T: 'a> {
    object: &'a mut T,
}

pub struct NodeEditor<N, ED> {
    inner: N,
    pub(crate) node_states: NodeStates,
    active_node: Option<NodeId>,
    drag_node: Option<NodeId>,
    creating_link: Option<LinkExtremity>,
    new_link: Option<(Output, InputSlot)>,
    pub show_left_pane: bool,
    left_pane_size: Option<f32>,
    pub show_top_pane: bool,
    pub show_connection_names: bool,
    pub(crate) scrolling: Scrolling,
    pub show_grid: bool,
    constant_editor: ED,
    error_stack: Vec<Box<error::Error>>,
}

enum LinkExtremity {
    Output(Output),
    Input(InputSlot),
}

#[derive(Copy, Clone)]
enum InputSlot {
    Transform(Input),
    Output(OutputId),
}

pub struct DstEditor<'t, T: 't + Clone, E: 't> {
    dst: DST<'t, T, E>,
    output_results: BTreeMap<OutputId, ComputeResult<T, E>>,
}

pub struct MacroEditor<'t, T: 't + Clone, E: 't> {
    macr: Macro<'t, T, E>,
}

pub struct NodeEditorApp<'t, T: 't + Clone, E: 't, ED> {
    main: NodeEditor<DstEditor<'t, T, E>, ED>,
    macros: BTreeMap<String, NodeEditor<MacroEditor<'t, T, E>, ED>>,
}

pub trait NodeEditable<'t, T: Clone + 't, E: 't>: Sized {
    type DSTHandle: DerefMut + Deref<Target = DST<'t, T, E>>;

    fn import<R: io::Read>(&self, r: R) -> Result<ImportSuccess<Self>, ImportError<E>>;
    fn export<W: io::Write>(&self, input: &ExportInput, w: &mut W) -> Result<(), ExportError>;
    fn with_dst(&mut self) -> Self::DSTHandle;
}
