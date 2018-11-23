use std::sync::Arc;

use cake::{self, Async, DSTError, Future, Task};

use editor::NodeEditor;

pub struct ComputationState<T, E> {
    previous_result: Option<Result<Arc<T>, Arc<DSTError<E>>>>,
    task: Task<Arc<T>, Arc<DSTError<E>>>,
    counter: u8,
}

impl<T, E> ComputationState<T, E> {}

impl<T, E, ED> NodeEditor<'static, T, E, ED>
where
    T: Clone + cake::VariantName + Send + Sync,
    E: Send + Sync,
{
    /// Compute output's result asynchonously.
    pub fn compute_output(
        &mut self,
        id: cake::OutputId,
    ) -> Option<Result<Arc<T>, Arc<DSTError<E>>>> {
        let dst = &self.dst;
        let cache = &mut self.cache;
        let state = self
            .output_results
            .entry(id)
            .or_insert_with(|| ComputationState {
                previous_result: None,
                task: dst.compute_next(id, cache),
                counter: 0,
            });

        const WRAP: u8 = 5;
        if state.counter % WRAP == 0 {
            match state.task.poll() {
                Ok(Async::Ready(t)) => {
                    state.previous_result = Some(Ok(t));
                    state.task = dst.compute_next(id, cache);
                }
                Ok(Async::NotReady) => (),
                Err(e) => {
                    state.previous_result = Some(Err(e));
                    state.task = dst.compute_next(id, cache);
                }
            };
        }
        if state.counter == WRAP - 1 {
            state.counter = 0;
        } else {
            state.counter += 1;
        }
        state.previous_result.clone()
    }
}
