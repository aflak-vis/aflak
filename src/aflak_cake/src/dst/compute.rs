use std::sync::Arc;

use rayon;

use super::super::ConvertibleVariants;
use cache::{Cache, CacheRef};
use dst::{DSTError, Output, OutputId, DST};
use future::Task;
use timed::Timed;
use variant_name::VariantName;

pub type SuccessOut<T> = Timed<Arc<T>>;
pub type ErrorOut<E> = Timed<Arc<DSTError<E>>>;
pub type NodeResult<T, E> = Result<SuccessOut<T>, ErrorOut<E>>;

impl<T, E> DST<'static, T, E>
where
    T: Clone + VariantName + ConvertibleVariants + Send + Sync,
    E: Send + Sync,
{
    /// Return the value out of the output given as argument.
    ///
    /// Distribute computation over several threads (if available).
    pub fn compute(
        &self,
        output_id: OutputId,
        cache: &mut Cache<T, DSTError<E>>,
    ) -> Task<SuccessOut<T>, ErrorOut<E>> {
        let t_indices = self.transforms.keys().cloned();
        cache.init(t_indices);

        if let Some(some_output) = self.outputs.get(&output_id) {
            if let Some(output) = some_output {
                let output = *output;
                let cache_ref = cache.get_ref();
                let dst = self.clone();
                Task::new(move || dst._compute(output, cache_ref))
            } else {
                Task::errored(Timed::from(Arc::new(DSTError::MissingOutputID(format!(
                    "Output ID {:?} is not attached!",
                    output_id
                )))))
            }
        } else {
            Task::errored(Timed::from(Arc::new(DSTError::MissingOutputID(format!(
                "Output ID {:?} not found!",
                output_id
            )))))
        }
    }

    fn _compute(&self, output: Output, cache: CacheRef<T, DSTError<E>>) -> NodeResult<T, E> {
        let meta = if let Some(meta) = self.transforms.get(&output.t_idx) {
            meta
        } else {
            return Err(Timed::from(Arc::new(DSTError::ComputeError(format!(
                "Transform {:?} not found!",
                output.t_idx
            )))));
        };

        let t_idx = output.t_idx;
        let index: usize = output.output_i.into();
        let updated_on = self.updated_on(t_idx);

        if let Some(result) = cache.compute(t_idx, updated_on, || {
            let deps = self
                .outputs_attached_to_transform(t_idx)
                .expect("Tranform not found!");

            let mut results = Vec::with_capacity(deps.len());
            for _ in 0..(deps.len()) {
                results.push(Err(Arc::new(DSTError::NothingDoneYet)));
            }
            let defaults = meta.defaults().to_vec();
            rayon::scope(|s| {
                for ((result, parent_output), default) in results.iter_mut().zip(deps).zip(defaults)
                {
                    let cache_clone = cache.clone();
                    s.spawn(move |_| {
                        *result = if let Some(output) = parent_output {
                            Timed::take_from_result(self._compute(output, cache_clone))
                        } else if let Some(default) = default {
                            Ok(Arc::new(default))
                        } else {
                            Err(Arc::new(DSTError::ComputeError(
                                "Missing dependency! Cannot compute.".to_owned(),
                            )))
                        }
                    })
                }
            });

            let t = meta.transform();
            let output_count = t.outputs().len();
            let mut op = t.start();
            for result in &results {
                match result {
                    Ok(ok) => op.feed(&**ok),
                    Err(e) => return vec![Err((*e).clone()); output_count],
                }
            }
            let mut out = Vec::with_capacity(output_count);
            for output in op.call() {
                out.push(output.map(Arc::new).map_err(|e| {
                    Arc::new(DSTError::InnerComputeError {
                        cause: e,
                        t_idx,
                        t_name: t.name(),
                    })
                }));
            }
            out
        }) {
            let timed = Timed::map(result, |mut result| result.remove(index));
            Timed::map_result(timed)
        } else {
            Err(Timed::from(Arc::new(DSTError::ComputeError(format!(
                "Cache is undergoing deletion! Cannot compute output {} now",
                output,
            )))))
        }
    }
}
