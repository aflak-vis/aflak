use std::mem;
use std::sync::{Arc, Mutex};

use cake::{self, DST};
use rayon;

use editor::NodeEditor;

#[derive(Debug)]
pub enum ComputationState<T> {
    NothingDone,
    RunningFirstTime,
    Running { previous_result: T },
    Completed { result: T },
}

pub type ComputeResult<T, E> = Arc<Mutex<ComputationState<Result<T, cake::DSTError<E>>>>>;

pub fn new_compute_result<T, E>() -> ComputeResult<T, E> {
    Arc::new(Mutex::new(ComputationState::NothingDone))
}

impl<T> ComputationState<T> {
    pub fn is_running(&self) -> bool {
        match *self {
            ComputationState::NothingDone => false,
            ComputationState::Completed { .. } => false,
            ComputationState::Running { .. } => true,
            ComputationState::RunningFirstTime => true,
        }
    }

    pub(crate) fn set_running(&mut self) {
        debug_assert!(!self.is_running(), "State is not running!");
        let interim = ComputationState::NothingDone;
        let prev = mem::replace(self, interim);
        let next = match prev {
            ComputationState::NothingDone => ComputationState::RunningFirstTime,
            ComputationState::Completed { result } => ComputationState::Running {
                previous_result: result,
            },
            _ => panic!("Expected computation state to not be running."),
        };
        mem::replace(self, next);
    }

    pub(crate) fn complete(&mut self, result: T) {
        *self = ComputationState::Completed { result };
    }

    pub fn result(&self) -> Option<&T> {
        match self {
            ComputationState::NothingDone => None,
            ComputationState::Completed { result } => Some(result),
            ComputationState::Running { previous_result } => Some(previous_result),
            ComputationState::RunningFirstTime => None,
        }
    }
}

impl<'t, T: 'static, E: 'static, ED> NodeEditor<'t, T, E, ED>
where
    T: Clone + cake::VariantName + Send + Sync,
    E: Send,
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

impl<'t, T, E, ED> NodeEditor<'t, T, E, ED>
where
    T: Clone,
{
    pub fn is_compute_running(&self) -> bool {
        self.output_results
            .values()
            .any(|result| result.lock().unwrap().is_running())
    }
}
