use crate::cake::NodeId;

/// Trait to get an i32 ID out of the implemented type.
pub trait GetId {
    fn id(&self) -> i32;
}

impl GetId for NodeId {
    fn id(&self) -> i32 {
        match *self {
            NodeId::Transform(t_idx) => t_idx.id() as i32,
            NodeId::Output(output_id) => -(output_id.id() as i32),
        }
    }
}
