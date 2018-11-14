use std::collections::BTreeMap;
use std::error;
use std::io;

use std::ops::DerefMut;

use cake::{self, InputSlot, Macro, MacroHandle, NodeId, Output, OutputId, Transformation, DST};

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

pub struct NodeEditor<'t, N, T: 't + Clone, E: 't, ED> {
    inner: N,
    addable_nodes: &'t [&'t Transformation<'t, T, E>],
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
}

enum LinkExtremity {
    Output(Output),
    Input(InputSlot),
}

pub struct DstEditor<'t, T: 't + Clone, E: 't> {
    dst: DST<'t, T, E>,
    output_results: BTreeMap<OutputId, ComputeResult<T, E>>,
}

pub struct MacroEditor<'t, T: 't + Clone, E: 't> {
    macr: Macro<'t, T, E>,
}

pub struct NodeEditorApp<'t, T: 't + Clone, E: 't, ED> {
    main: NodeEditor<'t, DstEditor<'t, T, E>, T, E, ED>,
    macros: BTreeMap<String, NodeEditor<'t, MacroEditor<'t, T, E>, T, E, ED>>,
    error_stack: Vec<Box<error::Error>>,
}

pub trait NodeEditable<'a, 't, T: Clone + 't, E: 't>: Sized {
    type DSTHandleMut: DerefMut<Target = DST<'t, T, E>>;

    fn import<R: io::Read>(&self, r: R) -> Result<ImportSuccess<Self>, ImportError<E>>;
    fn export<W: io::Write>(&self, input: &ExportInput, w: &mut W) -> Result<(), ExportError>;
    fn dst(&self) -> &DST<'t, T, E>;
    fn dst_mut(&'a mut self) -> Self::DSTHandleMut;
}

impl<'a, 't: 'a, T: Clone + 't, E: 't> NodeEditable<'a, 't, T, E> for DstEditor<'t, T, E> {
    type DSTHandleMut = &'a mut DST<'t, T, E>;

    fn import<R: io::Read>(&self, r: R) -> Result<ImportSuccess<Self>, ImportError<E>> {
        unimplemented!()
    }
    fn export<W: io::Write>(&self, input: &ExportInput, w: &mut W) -> Result<(), ExportError> {
        unimplemented!()
    }
    fn dst(&self) -> &DST<'t, T, E> {
        &self.dst
    }
    fn dst_mut(&'a mut self) -> Self::DSTHandleMut {
        &mut self.dst
    }
}

impl<'a, 't: 'a, T: Clone + 't, E: 't> NodeEditable<'a, 't, T, E> for MacroEditor<'t, T, E> {
    type DSTHandleMut = MacroHandle<'a, 't, T, E>;

    fn import<R: io::Read>(&self, r: R) -> Result<ImportSuccess<Self>, ImportError<E>> {
        unimplemented!()
    }
    fn export<W: io::Write>(&self, input: &ExportInput, w: &mut W) -> Result<(), ExportError> {
        unimplemented!()
    }
    fn dst(&self) -> &DST<'t, T, E> {
        &self.macr.dst()
    }
    fn dst_mut(&'a mut self) -> Self::DSTHandleMut {
        self.macr.dst_handle()
    }
}

/// ***************************************************************************/
/// Functions below are test to check that it compiles!!!                      /
impl<'a, 't, N, T, E, ED> NodeEditor<'t, N, T, E, ED>
where
    N: NodeEditable<'a, 't, T, E>,
    T: Clone,
{
    pub fn constant_node_value(&self, id: cake::TransformIdx) -> Option<&[T]> {
        self.inner.dst().get_transform(id).and_then(|t| {
            if let cake::Algorithm::Constant(ref constants) = t.algorithm {
                Some(constants.as_slice())
            } else {
                None
            }
        })
    }
}

impl<'a, 't, N, T, E, ED> NodeEditor<'t, N, T, E, ED>
where
    N: NodeEditable<'a, 't, T, E>,
    T: Clone + PartialEq,
{
    pub fn update_constant_node(&'a mut self, id: cake::TransformIdx, val: Vec<T>) {
        let mut dst = self.inner.dst_mut();
        let mut purge = false;
        if let Some(t) = dst.get_transform_mut(id) {
            if let cake::Algorithm::Constant(ref mut constants) = t.algorithm {
                for (c, val) in constants.iter_mut().zip(val.into_iter()) {
                    if *c != val {
                        *c = val;
                        purge = true;
                    }
                }
            }
        }
        if purge {
            dst.purge_cache_node(&cake::NodeId::Transform(id));
        }
    }
}
