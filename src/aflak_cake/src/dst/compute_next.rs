use std::sync::Arc;

use rayon;

use cache::{Cache, CacheRef};
use dst::{DSTError, Output, OutputId, DST};
use future::Task;
use variant_name::VariantName;

pub type NodeResult<T, E> = Result<Arc<T>, Arc<DSTError<E>>>;

impl<T, E> DST<'static, T, E>
where
    T: Clone + VariantName + Send + Sync,
    E: Send + Sync,
{
    pub fn compute_next(
        &self,
        output_id: OutputId,
        cache: &mut Cache<T, DSTError<E>>,
    ) -> Task<Arc<T>, Arc<DSTError<E>>> {
        let t_indices = self.transforms.keys().cloned();
        cache.init(t_indices);

        if let Some(some_output) = self.outputs.get(&output_id) {
            if let Some(output) = some_output {
                let output = *output;
                let cache_ref = cache.get_ref();
                let dst = self.clone();
                Task::new(move || dst._compute_next(output, cache_ref))
            } else {
                Task::errored(Arc::new(DSTError::MissingOutputID(format!(
                    "Output ID {:?} is not attached!",
                    output_id
                ))))
            }
        } else {
            Task::errored(Arc::new(DSTError::MissingOutputID(format!(
                "Output ID {:?} not found!",
                output_id
            ))))
        }
    }

    pub fn _compute_next(
        &self,
        output: Output,
        cache: CacheRef<T, DSTError<E>>,
    ) -> NodeResult<T, E> {
        let meta = if let Some(meta) = self.transforms.get(&output.t_idx) {
            meta
        } else {
            return Err(Arc::new(DSTError::ComputeError(format!(
                "Transform {:?} not found!",
                output.t_idx
            ))));
        };

        let t_idx = output.t_idx;
        let index: usize = output.output_i.into();
        let updated_on = self.updated_on(t_idx);

        if let Some(mut result) = cache.compute(t_idx, updated_on, || {
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
                            self._compute_next(output, cache_clone)
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
                out.push(
                    output
                        .map(Arc::new)
                        .map_err(|e| Arc::new(DSTError::InnerComputeError(e))),
                );
            }
            out
        }) {
            result.remove(index)
        } else {
            Err(Arc::new(DSTError::ComputeError(format!(
                "Cache is undergoing deletion! Cannot compute output {} now",
                output,
            ))))
        }
    }
}
