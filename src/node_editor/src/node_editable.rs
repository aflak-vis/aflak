use std::collections::BTreeMap;
use std::error;
use std::io;
use std::ops::DerefMut;

use ron::{de, ser};
use serde::{ser::Serializer, Deserialize, Serialize};

use cake::{
    self, DeserDST, InputSlot, Macro, MacroHandle, NodeId, Output, OutputId, Transformation, DST,
};

use compute::{self, ComputeResult};
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

    fn dst(&self) -> &DST<'t, T, E>;
    fn dst_mut(&'a mut self) -> Self::DSTHandleMut;
}

impl<'a, 't: 'a, T: Clone + 't, E: 't> NodeEditable<'a, 't, T, E> for DstEditor<'t, T, E> {
    type DSTHandleMut = &'a mut DST<'t, T, E>;

    fn dst(&self) -> &DST<'t, T, E> {
        &self.dst
    }
    fn dst_mut(&'a mut self) -> Self::DSTHandleMut {
        &mut self.dst
    }
}

impl<'a, 't: 'a, T: Clone + 't, E: 't> NodeEditable<'a, 't, T, E> for MacroEditor<'t, T, E> {
    type DSTHandleMut = MacroHandle<'a, 't, T, E>;

    fn dst(&self) -> &DST<'t, T, E> {
        &self.macr.dst()
    }
    fn dst_mut(&'a mut self) -> Self::DSTHandleMut {
        self.macr.dst_handle()
    }
}

impl<'t, N, T, E, ED> NodeEditor<'t, N, T, E, ED>
where
    T: Clone,
    N: Serialize,
{
    pub fn export_to_buf<W: io::Write>(&self, w: &mut W) -> Result<(), ExportError> {
        let serialized = ser::to_string_pretty(self, Default::default())?;
        w.write_all(serialized.as_bytes())?;
        w.flush()?;
        Ok(())
    }
}

#[derive(Serialize)]
pub struct SerialEditor<'e, N: 'e> {
    inner: &'e N,
    node_states: Vec<(&'e NodeId, &'e NodeState)>,
    scrolling: Vec2,
}

impl<'t, N, T, E, ED> Serialize for NodeEditor<'t, N, T, E, ED>
where
    N: Serialize,
    T: 't + Clone,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let ser = SerialEditor {
            inner: &self.inner,
            node_states: self.node_states.iter().collect(),
            scrolling: self.scrolling.get_current(),
        };
        ser.serialize(serializer)
    }
}

impl<'t, T, E> Serialize for DstEditor<'t, T, E>
where
    T: 't + Clone + Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.dst.serialize(serializer)
    }
}

#[derive(Deserialize)]
#[serde(bound(deserialize = "DN: Deserialize<'de>"))]
pub struct DeserEditor<DN> {
    inner: DN,
    node_states: Vec<(NodeId, NodeState)>,
    scrolling: Vec2,
}

impl<'t, N, T, E, ED> NodeEditor<'t, N, T, E, ED>
where
    T: Clone,
    N: Importable<ImportError<E>>,
{
    fn import_from_buf<R: io::Read>(&mut self, r: R) -> Result<(), ImportError<E>> {
        let deserialized: DeserEditor<N::Deser> = de::from_reader(r)?;

        // Set Ui node states
        self.node_states = {
            let mut node_states = NodeStates::new();
            for (node_id, state) in deserialized.node_states {
                node_states.insert(node_id, state);
            }
            node_states
        };
        // Set scrolling offset
        self.scrolling = Scrolling::new(deserialized.scrolling);
        self.inner.import(deserialized.inner)?;

        Ok(())
    }
}

pub trait Importable<Err>: Sized {
    type Deser: for<'de> serde::Deserialize<'de>;

    fn import(&mut self, Self::Deser) -> Result<(), Err>;
}

impl<'t, T, E> Importable<ImportError<E>> for DstEditor<'t, T, E>
where
    T: 'static + Clone + for<'de> Deserialize<'de> + cake::NamedAlgorithms<E> + cake::VariantName,
    E: 'static,
{
    type Deser = DeserEditor<DeserDST<T, E>>;

    fn import(&mut self, import: DeserEditor<DeserDST<T, E>>) -> Result<(), ImportError<E>> {
        // Replace DST. Wait for no computing to take place.
        use std::{thread, time};
        const SLEEP_INTERVAL_MS: u64 = 1;
        let sleep_interval = time::Duration::from_millis(SLEEP_INTERVAL_MS);
        println!("Import requested! Wait for pending compute tasks to complete...");
        let now = time::Instant::now();
        loop {
            if !self.is_compute_running() {
                println!("Starting import after {:?}", now.elapsed());
                break;
            } else {
                thread::sleep(sleep_interval);
            }
        }

        self.dst = import.inner.into()?;

        // Reset cache
        self.output_results = {
            let mut output_results = BTreeMap::new();
            for (output_id, _) in self.dst.outputs_iter() {
                output_results.insert(*output_id, compute::new_compute_result());
            }
            output_results
        };
        Ok(())
    }
}

impl<'t, T, E> DstEditor<'t, T, E>
where
    T: Clone,
{
    pub fn is_compute_running(&self) -> bool {
        self.output_results
            .values()
            .any(|result| result.lock().unwrap().is_running())
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
