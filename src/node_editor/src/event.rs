use std::error;

use cake::{InputSlot, NodeId, Output, Transform, TransformIdx, DST};

pub enum RenderEvent<T: 'static, E: 'static> {
    Connect(Output, InputSlot),
    AddTransform(&'static Transform<T, E>),
    CreateOutput,
    AddConstant(&'static str),
    SetConstant(TransformIdx, Box<T>),
    WriteDefaultInput {
        t_idx: TransformIdx,
        input_index: usize,
        val: Box<T>,
    },
    RemoveNode(NodeId),
}

impl<T, E> RenderEvent<T, E> {
    pub fn execute(self, dst: &mut DST<'static, T, E>) -> Result<(), Box<error::Error>>
    where
        T: Clone + cake::DefaultFor + cake::VariantName + cake::ConvertibleVariants,
    {
        match self {
            RenderEvent::Connect(output, input_slot) => match input_slot {
                InputSlot::Transform(input) => {
                    if let Err(e) = dst.connect(output, input) {
                        eprintln!("{:?}", e);
                        return Err(Box::new(e));
                    }
                }
                InputSlot::Output(output_id) => dst.update_output(output_id, output),
            },
            RenderEvent::AddTransform(t) => {
                dst.add_transform(t);
            }
            RenderEvent::CreateOutput => {
                dst.create_output();
            }
            RenderEvent::AddConstant(constant_type) => {
                let constant = cake::Transform::new_constant(T::default_for(constant_type));
                dst.add_owned_transform(constant);
            }
            RenderEvent::SetConstant(t_idx, val) => {
                if let Some(t) = dst.get_transform_mut(t_idx) {
                    t.set_constant(*val);
                } else {
                    eprintln!("Transform {:?} was not found.", t_idx);
                }
            }
            RenderEvent::WriteDefaultInput {
                t_idx,
                input_index,
                val,
            } => {
                if let Some(mut inputs) = dst.get_default_inputs_mut(t_idx) {
                    inputs.write(input_index, *val);
                } else {
                    eprintln!("Transform {:?} was not found.", t_idx);
                }
            }
            RenderEvent::RemoveNode(node_id) => {
                dst.remove_node(&node_id);
            }
        }
        Ok(())
    }
}
