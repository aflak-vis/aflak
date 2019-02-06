//! Data types for computational results.
use std::error;
use std::fmt;
use std::sync::Arc;

use boow::Bow;
use rayon;

use super::super::ConvertibleVariants;
use cache::{Cache, CacheRef};
use dst::{Input, Output, OutputId, TransformIdx, DST};
use future::Task;
use timed::Timed;
use variant_name::VariantName;

/// The successful result of a computation.
///
/// This is what the `DST::compute` method returns in case of success.
pub type SuccessOut<T> = Timed<Arc<T>>;

/// An computational error wrapped with `Timed` and a reference-counted pointer.
///
/// This is what the `DST::compute` method returns in case of error.
pub type ErrorOut<E> = Timed<Arc<ComputeError<E>>>;

/// The result of a computation.
///
/// This is what the `DST::compute` method returns.
pub type NodeResult<T, E> = Result<SuccessOut<T>, ErrorOut<E>>;

/// Represent a computational error.
#[derive(Debug)]
pub enum ComputeError<E> {
    UnattachedOutputID(OutputId),
    MissingOutputID(OutputId),
    MissingNode(TransformIdx),
    /// The output of a node should be attached to `input`, making the
    /// computation impossible.
    MissingDependency {
        input: Input,
        t_name: &'static str,
    },
    UnusableCache(Output),
    NothingDoneYet,
    /// Represent an error during computing, caused by user-defined
    /// transformations. This is usually caused by an unexpected input causing
    /// the calculation to abort.
    RuntimeError {
        cause: E,
        /// Where the error occurred.
        t_idx: TransformIdx,
        /// Name of function where error occurred.
        t_name: &'static str,
    },
    /// Represent an error that occurred previously in the stack. This error is
    /// used to rewind the stack and display clean error messages.
    ErrorStack {
        cause: Arc<ComputeError<E>>,
        /// Where the stack is.
        t_idx: TransformIdx,
        /// Name of function where the stack is.
        t_name: &'static str,
    },
}

impl<E: fmt::Display + fmt::Debug> error::Error for ComputeError<E> {
    fn description(&self) -> &'static str {
        "aflak_cake::ComputeError"
    }
}

impl<E: fmt::Display> fmt::Display for ComputeError<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::ComputeError::*;

        match self {
            UnattachedOutputID(output_id) => {
                write!(f, "Output #{} is not attached to a program", output_id.id())
            }
            MissingOutputID(output_id) => write!(f, "Output #{} is not found", output_id.id()),
            MissingNode(t_idx) => write!(f, "Node #{} not found", t_idx.id()),
            MissingDependency { input, t_name } => write!(
                f,
                "No output attached to {} in '{}', cannot compute",
                input, t_name
            ),
            UnusableCache(output) => write!(
                f,
                "Cache is unusable as it is undergoing deletion! Cannot compute {} now",
                output
            ),
            RuntimeError {
                cause,
                t_idx,
                t_name,
            } => write!(f, "{}\n    in node #{} {}", cause, t_idx.0, t_name),
            NothingDoneYet => write!(f, "Nothing done yet!"),
            ErrorStack {
                cause,
                t_idx,
                t_name,
            } => {
                // Unwind the stack and print it
                let mut stack = vec![(cause, t_idx, t_name)];
                let mut error = cause;
                while let ErrorStack {
                    cause,
                    t_idx,
                    t_name,
                } = &**error
                {
                    stack.push((cause, t_idx, t_name));
                    error = cause;
                }
                if let Some((root_cause, t_idx, t_name)) = stack.pop() {
                    write!(f, "{}\n    in node #{} {}", root_cause, t_idx.0, t_name)?;
                }
                while let Some((_, t_idx, t_name)) = stack.pop() {
                    write!(f, "\n    in node #{} {}", t_idx.0, t_name)?;
                }
                Ok(())
            }
        }
    }
}

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
        cache: &mut Cache<T, ComputeError<E>>,
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
                Task::errored(Timed::from(Arc::new(ComputeError::UnattachedOutputID(
                    output_id,
                ))))
            }
        } else {
            Task::errored(Timed::from(Arc::new(ComputeError::MissingOutputID(
                output_id,
            ))))
        }
    }

    fn _compute(&self, output: Output, cache: CacheRef<T, ComputeError<E>>) -> NodeResult<T, E> {
        let meta = if let Some(meta) = self.transforms.get(&output.t_idx) {
            meta
        } else {
            return Err(Timed::from(Arc::new(ComputeError::MissingNode(
                output.t_idx,
            ))));
        };

        let t = meta.transform();
        let t_idx = output.t_idx;
        let index: usize = output.output_i.into();
        let updated_on = self.updated_on(t_idx);

        if let Some(result) = cache.compute(t_idx, updated_on, || {
            let deps = self
                .outputs_attached_to_transform(t_idx)
                .expect("Tranform not found!");

            let mut results = Vec::with_capacity(deps.len());
            for _ in 0..(deps.len()) {
                results.push(Err(Arc::new(ComputeError::NothingDoneYet)));
            }
            let defaults = meta.defaults().to_vec();
            rayon::scope(|s| {
                for (i, ((result, parent_output), default)) in
                    results.iter_mut().zip(deps).zip(defaults).enumerate()
                {
                    let cache_clone = cache.clone();
                    s.spawn(move |_| {
                        *result = if let Some(output) = parent_output {
                            Timed::take_from_result(self._compute(output, cache_clone))
                        } else if let Some(default) = default {
                            Ok(Arc::new(default))
                        } else {
                            Err(Arc::new(ComputeError::MissingDependency {
                                input: Input::new(t_idx, i),
                                t_name: t.name(),
                            }))
                        }
                    })
                }
            });

            let output_count = t.outputs().len();
            let mut op = t.start();
            for result in &results {
                match result {
                    Ok(ok) => op.feed(&**ok),
                    Err(e) => {
                        let error_stack = ComputeError::ErrorStack {
                            cause: e.clone(),
                            t_idx,
                            t_name: t.name(),
                        };
                        return vec![Err(Arc::new(error_stack)); output_count];
                    }
                }
            }
            let mut out = Vec::with_capacity(output_count);
            for output in op.call() {
                out.push(output.map(Arc::new).map_err(|e| {
                    Arc::new(ComputeError::RuntimeError {
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
            Err(Timed::from(Arc::new(ComputeError::UnusableCache(output))))
        }
    }

    /// Update default input values with the current value in the cache
    pub fn update_defaults_from_cache(&mut self, cache: &Cache<T, ComputeError<E>>) {
        // Iterate over inputs with a default value AND connected to an input
        for (output, input_list) in self.edges.iter() {
            if let Some(Ok(result)) = cache.get(output) {
                for input in &input_list.inputs {
                    if let Some(meta) = self.transforms.get_mut(&input.t_idx) {
                        if let Some(Some(default)) = meta.input_defaults.get_mut(input.index()) {
                            let expected_type = default.variant_name();
                            let incoming_type = result.variant_name();
                            if let Some(converted) =
                                T::convert(incoming_type, expected_type, &*result)
                            {
                                // Do not update updated_on, as default value not used
                                // for computing, only showing
                                // If updated_on was updated, computation would never stop!
                                *default = match converted {
                                    Bow::Borrowed(v) => v.clone(),
                                    Bow::Owned(v) => v,
                                };
                            }
                        }
                    }
                }
            }
        }
    }
}
