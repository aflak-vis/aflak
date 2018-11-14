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
    self, DSTGuard, DSTGuardMut, DeserDST, InputSlot, Macro, MacroEvaluationError, MacroHandle,
    NodeId, Output, OutputId, Transformation, DST,
};

use compute::{self, ComputeResult};
use editor::{DeserEditor, NodeEditor};
use export::{ExportError, ImportError};
use node_state::{NodeState, NodeStates};
use scrolling::Scrolling;
use vec2::Vec2;

pub struct DstEditor<'t, T: 't + Clone, E: 't> {
    pub(crate) dst: DST<'t, T, E>,
    output_results: BTreeMap<OutputId, ComputeResult<T, E>>,
}

impl<'t, T: 't + Clone, E: 't> Default for DstEditor<'t, T, E> {
    fn default() -> Self {
        Self {
            dst: DST::default(),
            output_results: BTreeMap::default(),
        }
    }
}

impl<'t, T: 't + Clone, E: 't> DstEditor<'t, T, E> {
    pub fn from_dst(dst: DST<'t, T, E>) -> Self {
        let mut output_results = BTreeMap::new();
        for (output_id, _) in dst.outputs_iter() {
            output_results.insert(*output_id, compute::new_compute_result());
        }
        DstEditor {
            dst,
            output_results,
        }
    }
}

pub struct MacroEditor<'t, T: 't + Clone, E: 't> {
    macr: Macro<'t, T, E>,
}

pub struct NodeEditorApp<'t, T: 't + Clone, E: 't, ED> {
    main: NodeEditor<'t, DstEditor<'t, T, E>, T, E, ED>,
    macros: BTreeMap<String, NodeEditor<'t, MacroEditor<'t, T, E>, T, E, ED>>,
    error_stack: Vec<Box<error::Error>>,
}

pub trait NodeEditable<'t, T: Clone + 't, E: 't>: Sized {
    // type DSTHandle: Deref<Target = DST<'t, T, E>>;
    // type DSTHandleMut: DerefMut<Target = DST<'t, T, E>>;

    fn dst(&self) -> DSTGuard<'_, 't, T, E>;
    fn dst_mut(&mut self) -> DSTGuardMut<'_, 't, T, E>;

    fn create_output(&mut self) -> OutputId;
}

impl<'t, T: Clone + 't, E: 't> NodeEditable<'t, T, E> for DstEditor<'t, T, E> {
    // type DSTHandle = &'a DST<'t, T, E>;
    // type DSTHandleMut = &'a mut DST<'t, T, E>;

    fn dst(&self) -> DSTGuard<'_, 't, T, E> {
        DSTGuard::StandAlone(&self.dst)
    }
    fn dst_mut(&mut self) -> DSTGuardMut<'_, 't, T, E> {
        DSTGuardMut::StandAlone(&mut self.dst)
    }
    fn create_output(&mut self) -> OutputId {
        let id = self.dst.create_output();
        self.output_results
            .insert(id, compute::new_compute_result());
        id
    }
}

impl<'t, T: Clone + 't, E: 't> NodeEditable<'t, T, E> for MacroEditor<'t, T, E> {
    // type DSTHandle = GuardRef<'a, DST<'t, T, E>>;
    // type DSTHandleMut = MacroHandle<'a, 't, T, E>;

    fn dst(&self) -> DSTGuard<'_, 't, T, E> {
        self.macr.dst()
    }
    fn dst_mut(&mut self) -> DSTGuardMut<'_, 't, T, E> {
        self.macr.dst_mut()
    }
    fn create_output(&mut self) -> OutputId {
        self.macr.dst_mut().create_output()
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
