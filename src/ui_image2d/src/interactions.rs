use std::collections::{btree_map, BTreeMap};

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f32),
    Float2([f32; 2]),
    Float3([f32; 3]),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Interaction {
    HorizontalLine(f32),
}

pub struct Interactions(BTreeMap<InteractionId, Interaction>);

impl Interactions {
    pub(crate) fn new() -> Self {
        Interactions(BTreeMap::new())
    }

    pub(crate) fn value_iter(&self) -> ValueIter {
        ValueIter(self.0.iter())
    }

    pub(crate) fn interaction_iter(&self) -> InteractionIter {
        InteractionIter(self.0.iter())
    }
}

impl Interaction {
    pub(crate) fn value(&self) -> Value {
        match self {
            Interaction::HorizontalLine(height) => Value::Float(*height),
        }
    }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct InteractionId(usize);

#[derive(Clone, Debug)]
pub struct ValueIter<'a>(btree_map::Iter<'a, InteractionId, Interaction>);

#[derive(Clone, Debug)]
pub struct InteractionIter<'a>(btree_map::Iter<'a, InteractionId, Interaction>);

impl<'a> Iterator for ValueIter<'a> {
    type Item = (&'a InteractionId, Value);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(id, inter)| (id, inter.value()))
    }
}

impl<'a> Iterator for InteractionIter<'a> {
    type Item = (&'a InteractionId, &'a Interaction);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}
