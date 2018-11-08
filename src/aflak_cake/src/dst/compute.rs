use std::sync::RwLock;

use rayon;
use variant_name::VariantName;

use dst::node::NodeId;
use dst::{DSTError, Output, OutputId, DST};

impl<'t, T: 't, E: 't> DST<'t, T, E>
where
    T: Clone + VariantName + Send + Sync,
    E: Send,
{
    fn _compute(&self, output: Output) -> Result<T, DSTError<E>> {
        let meta = self.transforms.get(&output.t_idx).ok_or_else(|| {
            DSTError::ComputeError(format!("Tranform {:?} not found!", output.t_idx))
        })?;
        let t = meta.transform();
        let output_cache_lock = self.cache.get(&output).expect("Get output cache");
        {
            let output_cache = output_cache_lock.read().unwrap();
            if let Some(ref cache) = *output_cache {
                return Ok(cache.clone());
            }
        }
        let deps = self
            .outputs_attached_to_transform(output.t_idx)
            .ok_or_else(|| {
                DSTError::ComputeError(format!("Transform {:?} not found!", output.t_idx))
            })?;
        let mut op = t.start();
        let mut results = Vec::with_capacity(deps.len());
        for _ in 0..(deps.len()) {
            results.push(Err(DSTError::NothingDoneYet));
        }
        let defaults = meta.defaults().to_vec();
        rayon::scope(|s| {
            for ((result, parent_output), default) in results.iter_mut().zip(deps).zip(defaults) {
                s.spawn(move |_| {
                    *result = if let Some(output) = parent_output {
                        self._compute(output)
                    } else if let Some(default) = default {
                        Ok(default)
                    } else {
                        Err(DSTError::ComputeError(
                            "Missing dependency! Cannot compute.".to_owned(),
                        ))
                    }
                })
            }
        });
        for result in results {
            op.feed(result?);
        }
        match op.call().nth(output.output_i.into()) {
            None => Err(DSTError::ComputeError(
                "No nth output received. This is a bug!".to_owned(),
            )),
            Some(result) => {
                if let Ok(ref result) = result {
                    let mut cache = output_cache_lock.write().unwrap();
                    *cache = Some(result.clone())
                }
                result.map_err(|err| DSTError::InnerComputeError(err))
            }
        }
    }

    /// Return the result of the computation to the output given as argument.
    ///
    /// If possible, computation is distributed on several threads.
    pub fn compute(&self, output_id: OutputId) -> Result<T, DSTError<E>> {
        self.outputs
            .get(&output_id)
            .ok_or_else(|| {
                DSTError::MissingOutputID(format!("Output ID {:?} not found!", output_id))
            }).and_then(|output| {
                output.ok_or_else(|| {
                    DSTError::MissingOutputID(format!("Output ID {:?} is not attached!", output_id))
                })
            }).and_then(|output| self._compute(output))
    }
}

impl<'t, T, E> DST<'t, T, E>
where
    T: 't + Clone,
    E: 't,
{
    /// Purge all cache in the given output and all its children.
    pub(crate) fn purge_cache(&mut self, output: Output) {
        self.cache.insert(output, RwLock::new(None));
        let inputs: Option<Vec<_>> = self
            .inputs_attached_to(&output)
            .map(|inputs| inputs.map(|input| *input))
            .map(Iterator::collect);
        if let Some(inputs) = inputs {
            for input in inputs {
                let outputs = self.outputs_of_transformation(input.t_idx);
                if let Some(outputs) = outputs {
                    for output in outputs {
                        self.purge_cache(output);
                    }
                }
            }
        }
    }

    /// Purge cache for specified node.
    pub fn purge_cache_node(&mut self, node_id: &NodeId) {
        match node_id {
            &NodeId::Output(ref output_id) => {
                let output = {
                    if let Some(Some(output)) = self.outputs.get(output_id) {
                        *output
                    } else {
                        return;
                    }
                };
                self.purge_cache(output);
            }
            &NodeId::Transform(t_idx) => {
                if let Some(outputs) = self.outputs_of_transformation(t_idx) {
                    for output in outputs {
                        self.purge_cache(output);
                    }
                }
            }
        }
    }
}
