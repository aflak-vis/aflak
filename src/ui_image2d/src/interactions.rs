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
    HorizontalLine(HorizontalLine),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct HorizontalLine {
    pub height: f32,
    pub moving: bool,
}

impl HorizontalLine {
    pub fn new(height: f32) -> Self {
        Self {
            height,
            moving: false,
        }
    }
}

/// Record all interactions.
///
/// Contains a counter that counts the number of interactions inserted.
/// The counter is used to set a unique ID to all interactions that ever existed.
pub struct Interactions(BTreeMap<InteractionId, Interaction>, usize);

impl Interactions {
    pub(crate) fn new() -> Self {
        Interactions(BTreeMap::new(), 0)
    }

    pub(crate) fn value_iter(&self) -> ValueIter {
        ValueIter(self.0.iter())
    }

    pub(crate) fn iter_mut(&mut self) -> InteractionIterMut {
        InteractionIterMut(self.0.iter_mut())
    }

    pub(crate) fn insert(&mut self, interaction: Interaction) -> Option<Interaction> {
        let new_id = InteractionId(self.1 + 1);
        self.1 += 1;
        self.0.insert(new_id, interaction)
    }

    pub(crate) fn remove(&mut self, id: &InteractionId) -> Option<Interaction> {
        self.0.remove(id)
    }
}

impl Interaction {
    pub(crate) fn value(&self) -> Value {
        match self {
            Interaction::HorizontalLine(HorizontalLine { height, .. }) => Value::Float(*height),
        }
    }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct InteractionId(usize);

impl InteractionId {
    /// Get ImGui unique ID
    pub fn id(&self) -> i32 {
        self.0 as i32
    }
}

#[derive(Clone, Debug)]
pub struct ValueIter<'a>(btree_map::Iter<'a, InteractionId, Interaction>);

#[derive(Debug)]
pub struct InteractionIterMut<'a>(btree_map::IterMut<'a, InteractionId, Interaction>);

impl<'a> Iterator for ValueIter<'a> {
    type Item = (&'a InteractionId, Value);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(id, inter)| (id, inter.value()))
    }
}

impl<'a> Iterator for InteractionIterMut<'a> {
    type Item = (&'a InteractionId, &'a mut Interaction);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}
