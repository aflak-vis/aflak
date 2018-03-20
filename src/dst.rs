use transform::{Transformation, TypeContent};

#[derive(Clone)]
pub struct DST<T: TypeContent> {
    t: Transformation<T>,
    output_connections: Vec<OutputConnection<T>>,
}

#[derive(Clone)]
pub enum OutputConnection<T: TypeContent> {
    Drop,
    Out,
    Child(DST<T>),
}

impl<T: TypeContent> DST<T> {
    fn new(t: Transformation<T>) -> Self {
        let mut connections = Vec::new();
        connections.resize(t.output.len(), OutputConnection::Drop);
        Self {
            t,
            output_connections: connections,
        }
    }
}
