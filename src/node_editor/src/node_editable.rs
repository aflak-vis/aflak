use std::collections::BTreeMap;
use std::error;
use std::fs;
use std::io;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::path::Path;

use ron::{de, ser};
use serde::{ser::Serializer, Deserialize, Serialize};

use cake::{
    self, DeserDST, GuardRef, InputSlot, Macro, MacroEvaluationError, MacroHandle, NodeId, Output,
    OutputId, Transformation, DST,
};

use compute::{self, ComputeResult};
use export::{ExportError, ImportError};
use node_state::{NodeState, NodeStates};
use scrolling::Scrolling;
use vec2::Vec2;

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
    type DSTHandle: Deref<Target = DST<'t, T, E>>;
    type DSTHandleMut: DerefMut<Target = DST<'t, T, E>>;

    fn dst(&'a self) -> Self::DSTHandle;
    fn dst_mut(&'a mut self) -> Self::DSTHandleMut;
}

impl<'a, 't: 'a, T: Clone + 't, E: 't> NodeEditable<'a, 't, T, E> for DstEditor<'t, T, E> {
    type DSTHandle = &'a DST<'t, T, E>;
    type DSTHandleMut = &'a mut DST<'t, T, E>;

    fn dst(&self) -> &DST<'t, T, E> {
        &self.dst
    }
    fn dst_mut(&mut self) -> &mut DST<'t, T, E> {
        &mut self.dst
    }
}

impl<'a, 't: 'a, T: Clone + 't, E: 't> NodeEditable<'a, 't, T, E> for MacroEditor<'t, T, E> {
    type DSTHandle = GuardRef<'a, DST<'t, T, E>>;
    type DSTHandleMut = MacroHandle<'a, 't, T, E>;

    fn dst(&self) -> GuardRef<DST<'t, T, E>> {
        self.macr.dst()
    }
    fn dst_mut(&'a mut self) -> MacroHandle<'a, 't, T, E> {
        self.macr.dst_mut()
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

    fn import_from_file<P: AsRef<Path>>(&mut self, file_path: P) -> Result<(), ImportError<E>> {
        let f = fs::File::open(file_path)?;
        self.import_from_buf(f)
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

impl<'t, T: 'static, E: 'static> DstEditor<'t, T, E>
where
    T: Clone + cake::VariantName + Send + Sync,
    E: Send + From<MacroEvaluationError<E>>,
{
    /// Compute output's result asynchonously.
    ///
    /// `self` should live longer as long as computing is not finished.
    /// If not, you'll get undefined behavior!
    pub unsafe fn compute_output(&self, id: cake::OutputId) -> ComputeResult<T, E> {
        let result_lock = &self.output_results[&id];
        let mut result = result_lock.lock().unwrap();
        if result.is_running() {
            // Currently computing... Nothing to do
            drop(result);
        } else {
            result.set_running();
            drop(result);
            let result_lock_clone = result_lock.clone();
            // Extend dst's lifetime
            let dst: &'static DST<T, E> = mem::transmute(&self.dst);
            rayon::spawn(move || {
                let result = dst.compute(id);
                result_lock_clone.lock().unwrap().complete(result);
            });
        }
        result_lock.clone()
    }
}
