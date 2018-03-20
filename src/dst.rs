use std::sync::{Arc, Mutex};
use std::str::FromStr;

use transform::{Transformation, TypeContent};

#[derive(Clone)]
pub struct DST<T: TypeContent> {
    head: Arc<Mutex<Node<T>>>,
}

struct Node<T: TypeContent> {
    t: Transformation<T>,
    outputs: Vec<Option<usize>>,
    children: Vec<Option<Arc<Mutex<Node<T>>>>>,
}

impl<T: TypeContent> DST<T> {
    pub fn new(t: Transformation<T>) -> Self {
        let len = t.output.len();
        Self {
            head: Arc::new(Mutex::new(Node {
                t,
                outputs: vec![None; len],
                children: vec![None; len],
            })),
        }
    }

    pub fn attach_child_to(&mut self, output_i: usize, dst: &DST<T>) {
        let mut node = self.head.lock().unwrap();
        node.children[output_i] = Some(dst.head.clone());
    }

    pub fn detach_child(&mut self, output_i: usize) -> Option<DST<T>> {
        let mut node = self.head.lock().unwrap();
        node.children[output_i].take().map(|child| {
            Self { head: child.clone() }
        })
    }

    pub fn attach_out(&mut self, output_i: usize, out_id: usize) {
        let mut node = self.head.lock().unwrap();
        node.outputs[output_i] = Some(out_id);
    }

    pub fn detach_out(&mut self, output_i: usize) {
        let mut node = self.head.lock().unwrap();
        node.outputs[output_i] = None;
    }
}

pub struct ParseDSTError;

impl<T: TypeContent> FromStr for DST<T> {
    type Err = ParseDSTError;
    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        unimplemented!()
    }
}
