use cake::{
    self,
    compute::{ErrorOut, NodeResult, SuccessOut},
    Async, Future, Task,
};

use editor::NodeEditor;

pub struct ComputationState<T, E> {
    previous_result: Option<NodeResult<T, E>>,
    task: Task<SuccessOut<T>, ErrorOut<E>>,
    counter: u8,
}

impl<T, E, ED> NodeEditor<'static, T, E, ED>
where
    T: Clone + cake::VariantName + Send + Sync,
    E: Send + Sync,
{
    /// Compute output's result asynchonously.
    pub fn compute_output(&mut self, id: cake::OutputId) -> Option<NodeResult<T, E>> {
        let dst = &self.dst;
        let cache = &mut self.cache;
        let state = self
            .output_results
            .entry(id)
            .or_insert_with(|| ComputationState {
                previous_result: None,
                task: dst.compute(id, cache),
                counter: 1,
            });

        const WRAP: u8 = 5;
        if state.counter % WRAP == 0 {
            match state.task.poll() {
                Ok(Async::Ready(t)) => {
                    state.previous_result = Some(Ok(t));
                    state.task = dst.compute(id, cache);
                }
                Ok(Async::NotReady) => (),
                Err(e) => {
                    state.previous_result = Some(Err(e));
                    state.task = dst.compute(id, cache);
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
