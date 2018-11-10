use std::borrow::Cow;

use super::{Input, InputSlot, OutputId, DST};

#[derive(Debug)]
pub struct Macro<'t, T: Clone + 't, E: 't> {
    inputs: Vec<(InputSlotRef, Option<T>)>,
    dst: DST<'t, T, E>,
}

/// TODO: Move that to UI!
// pub struct Macros<'t, T: Clone + 't, E: 't> {
//     macros: HashMap<String, Macro<'t, T, E>>,
// }

impl<'t, T: Clone + 't, E: 't> Macro<'t, T, E> {
    pub fn new(dst: DST<'t, T, E>) -> Self {
        Self {
            inputs: Self::find_inputs(&dst)
                .into_iter()
                .map(|input| (input, None))
                .collect(),
            dst,
        }
    }

    pub fn call(&self, _args: Vec<Cow<T>>) -> Vec<Result<T, E>> {
        unimplemented!()
    }

    fn find_inputs(dst: &DST<'t, T, E>) -> Vec<InputSlotRef> {
        let mut inputs = vec![];
        for (output, input_slot) in dst.input_slots_iter() {
            let no_output = output.is_none();
            let default_value = if let InputSlot::Transform(input) = input_slot {
                let t_idx = input.t_idx;
                let t = dst.get_transform(t_idx).unwrap();
                let index = input.index();
                if t.input[index].1.is_some() {
                    true
                } else {
                    false
                }
            } else {
                false
            };
            if no_output && !default_value {
                let input_slot = InputSlotRef::from(input_slot);
                inputs.push(input_slot);
            }
        }
        inputs
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
enum InputSlotRef {
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
