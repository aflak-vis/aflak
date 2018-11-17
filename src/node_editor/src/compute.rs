use std::mem;
use std::sync::{Arc, Mutex};

use cake;

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
