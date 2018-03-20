use std::sync::{Arc, Mutex};

use transform::{Transformation, TypeContent};

#[derive(Clone)]
pub struct DST<T: TypeContent> {
    head: Arc<Mutex<Node<T>>>,
}

struct Node<T: TypeContent> {
    t: Transformation<T>,
    output_connections: Vec<OutputConnection<T>>,
}

#[derive(Clone)]
enum OutputConnection<T: TypeContent> {
    Drop,
    Out(usize),
    Child(Arc<Mutex<Node<T>>>),
}

impl<T: TypeContent> DST<T> {
    pub fn new(t: Transformation<T>) -> Self {
        let mut connections = Vec::new();
        connections.resize(t.output.len(), OutputConnection::Drop);
        Self {
            head: Arc::new(Mutex::new(Node {
                t,
                output_connections: connections,
            })),
        }
    }

    pub fn attach_to(&mut self, output_i: usize, dst: &DST<T>) {
        let mut node = self.head.lock().unwrap();
        node.output_connections[output_i] = OutputConnection::Child(dst.head.clone());
    }

    pub fn detach(&mut self, output_i: usize) -> Option<DST<T>> {
        let mut node = self.head.lock().unwrap();
        let mut ret = None;
        node.output_connections[output_i] = match node.output_connections[output_i] {
            OutputConnection::Child(ref mut child) => {
                ret = Some(Self { head: child.clone() });
                OutputConnection::Drop
            },
            _ => OutputConnection::Drop,
        };
        ret
    }

    pub fn set_out(&mut self, output_i: usize, out_id: usize) {
        let mut node = self.head.lock().unwrap();
        node.output_connections[output_i] = OutputConnection::Out(out_id);
    }
}
