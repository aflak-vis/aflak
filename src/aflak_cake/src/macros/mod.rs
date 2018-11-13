use std::borrow::Cow;
use std::ops::{Deref, DerefMut};

use super::{DSTError, Input, InputSlot, OutputId, VariantName, DST};

mod error;
pub use self::error::MacroEvaluationError;
use transform::TypeId;

#[derive(Debug)]
pub struct Macro<'t, T: Clone + 't, E: 't> {
    inputs: Vec<(InputSlotRef, TypeId, Option<T>)>,
    dst: DST<'t, T, E>,
}

impl<'t, T: Clone + 't, E: 't> Macro<'t, T, E> {
    pub fn new(dst: DST<'t, T, E>) -> Self {
        Self {
            inputs: Self::find_inputs(&dst)
                .into_iter()
                .map(|(input, type_id)| (input, type_id, None))
                .collect(),
            dst,
        }
    }

    pub fn inputs(&self) -> &[(InputSlotRef, TypeId, Option<T>)] {
        &self.inputs
    }

    pub fn outputs(&self) -> Vec<TypeId> {
        self.dst
            .outputs_iter()
            .collect::<::std::collections::BTreeMap<_, _>>()
            .into_iter()
            .map(|(_, some_output)| {
                if let Some(output) = some_output {
                    let t = self.dst.get_transform(output.t_idx).unwrap();
                    t.output[output.index()]
                } else {
                    // Not type can be defined as nothing is attached to this output
                    ""
                }
            }).collect()
    }

    fn find_inputs(dst: &DST<'t, T, E>) -> Vec<(InputSlotRef, TypeId)> {
        let mut inputs = vec![];
        for (output, input_slot) in dst.input_slots_iter() {
            let no_output = output.is_none();
            let (default_value, type_id) = if let InputSlotRef::Transform(input) = input_slot {
                let t_idx = input.t_idx;
                let t = dst.get_transform(t_idx).unwrap();
                let index = input.index();
                let input = &t.input[index];
                (input.1.is_some(), input.0)
            } else {
                (false, "")
            };
            if no_output && !default_value {
                let input_slot = InputSlotRef::from(input_slot);
                inputs.push((input_slot, type_id));
            }
        }
        inputs
    }

    pub fn dst_handle<'m>(&'m mut self) -> MacroHandle<'m, 't, T, E> {
        MacroHandle { macr: self }
    }
}

pub struct MacroHandle<'m, 't: 'm, T: Clone + 't, E: 't> {
    macr: &'m mut Macro<'t, T, E>,
}

impl<'m, 't: 'm, T: Clone + 't, E: 't> Deref for MacroHandle<'m, 't, T, E> {
    type Target = DST<'t, T, E>;

    fn deref(&self) -> &Self::Target {
        &self.macr.dst
    }
}

impl<'m, 't: 'm, T: Clone + 't, E: 't> DerefMut for MacroHandle<'m, 't, T, E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.macr.dst
    }
}

impl<'m, 't: 'm, T: Clone + 't, E: 't> Drop for MacroHandle<'m, 't, T, E> {
    fn drop(&mut self) {
        println!("Drop MacroHandle. Need to recompute inputs")
        // TODO
    }
}

impl<'t, T, E> Macro<'t, T, E>
where
    T: Clone + VariantName + Send + Sync + 't,
    E: 't + Send + From<MacroEvaluationError<E>>,
{
    pub fn call(&self, args: Vec<Cow<T>>) -> Vec<Result<T, E>> {
        let inputs = self
            .inputs
            .iter()
            .map(|(input_slot, _, _)| input_slot)
            .zip(args.into_iter())
            .collect::<Vec<_>>();
        self.dst
            .outputs_iter()
            .map(|(id, _)| *id)
            .collect::<::std::collections::BTreeSet<_>>()
            .into_iter()
            .map(|output_id| {
                self.dst
                    .compute_macro(output_id, &inputs)
                    .map_err(|e| From::from(MacroEvaluationError::DSTError(e)))
            }).collect()
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum InputSlotRef {
    Transform(Input),
    Output(OutputId),
}

impl<'a> From<InputSlot<'a>> for InputSlotRef {
    fn from(slot: InputSlot<'a>) -> Self {
        match slot {
            InputSlot::Transform(input) => InputSlotRef::Transform(*input),
            InputSlot::Output(output_id) => InputSlotRef::Output(*output_id),
        }
    }
}
