use std::sync::Arc;

use boow::Bow;

use super::{TransformInputSlot, TypeId};
use compute::ComputeError;

#[derive(Clone)]
pub struct MacroHandle;

/// TODO
impl MacroHandle {
    pub fn name(&self) -> String {
        "Macro".to_owned()
    }

    pub fn input_types(&self) -> Vec<TypeId> {
        vec![]
    }

    pub fn outputs(&self) -> Vec<TypeId> {
        vec![]
    }

    pub fn inputs<T>(&self) -> Vec<TransformInputSlot<T>> {
        vec![]
    }

    pub fn defaults<T>(&self) -> Vec<Option<T>> {
        vec![]
    }

    pub fn id(&self) -> usize {
        0
    }

    pub fn call<T, E>(&self, _args: Vec<Bow<'_, T>>) -> Vec<Result<T, Arc<ComputeError<E>>>> {
        vec![]
    }
}
