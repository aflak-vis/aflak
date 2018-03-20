use std::sync::Arc;

use transform::{Transformation, TypeContent};

#[derive(Clone)]
pub struct DST<T: TypeContent> {
    head: Arc<Node<T>>,
}

struct Node<T: TypeContent> {
    t: Transformation<T>,
    output_connections: Vec<OutputConnection<T>>,
}

#[derive(Clone)]
enum OutputConnection<T: TypeContent> {
    Drop,
    Out,
    Child(Arc<Node<T>>),
}

impl<T: TypeContent> DST<T> {
    pub fn new(t: Transformation<T>) -> Self {
        let mut connections = Vec::new();
        connections.resize(t.output.len(), OutputConnection::Drop);
        Self {
            head: Arc::new(Node {
                t,
                output_connections: connections,
            }),
        }
    }
}
