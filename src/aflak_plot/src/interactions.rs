use std::collections::{btree_map, BTreeMap};

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f32),
    Float2([f32; 2]),
    Float3([f32; 3]),
    FinedGrainedROI(Vec<(usize, usize)>),
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Integer(v)
    }
}
impl From<f32> for Value {
    fn from(v: f32) -> Self {
        Value::Float(v)
    }
}
impl From<[f32; 2]> for Value {
    fn from(v: [f32; 2]) -> Self {
        Value::Float2(v)
    }
}
impl From<[f32; 3]> for Value {
    fn from(v: [f32; 3]) -> Self {
        Value::Float3(v)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Interaction {
    HorizontalLine(HorizontalLine),
    VerticalLine(VerticalLine),
    FinedGrainedROI(FinedGrainedROI),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct HorizontalLine {
    pub height: f32,
    pub moving: bool,
}
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct VerticalLine {
    pub x_pos: f32,
    pub moving: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct FinedGrainedROI {
    pub(crate) id: usize,
    pub pixels: Vec<(usize, usize)>,
}

impl HorizontalLine {
    pub fn new(height: f32) -> Self {
        Self {
            height,
            moving: false,
        }
    }
}

impl VerticalLine {
    pub fn new(x_pos: f32) -> Self {
        Self {
            x_pos,
            moving: false,
        }
    }
}

impl FinedGrainedROI {
    pub fn new(id: usize) -> Self {
        Self { id, pixels: vec![] }
    }
}

/// Record all interactions.
///
/// Contains a counter that counts the number of interactions inserted.
/// The counter is used to set a unique ID to all interactions that ever existed.
#[derive(Debug)]
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

    pub(crate) fn remove(&mut self, id: InteractionId) -> Option<Interaction> {
        self.0.remove(&id)
    }

    pub(crate) fn any_moving(&self) -> bool {
        self.0.iter().any(|(_, interaction)| match interaction {
            Interaction::HorizontalLine(HorizontalLine { moving, .. }) => *moving,
            Interaction::VerticalLine(VerticalLine { moving, .. }) => *moving,
            Interaction::FinedGrainedROI(..) => false,
        })
    }
}

impl Interaction {
    pub(crate) fn value(&self) -> Value {
        match self {
            Interaction::HorizontalLine(HorizontalLine { height, .. }) => Value::Float(*height),
            Interaction::VerticalLine(VerticalLine { x_pos, .. }) => Value::Float(*x_pos),
            Interaction::FinedGrainedROI(FinedGrainedROI { pixels, .. }) => {
                Value::FinedGrainedROI(pixels.clone())
            }
        }
    }

    pub fn set_value<V: Into<Value>>(&mut self, value: V) -> Result<(), String> {
        let value = value.into();
        match (self, &value) {
            (
                Interaction::HorizontalLine(HorizontalLine { ref mut height, .. }),
                Value::Float(f),
            ) => {
                *height = *f;
                Ok(())
            }
            (Interaction::VerticalLine(VerticalLine { ref mut x_pos, .. }), Value::Float(f)) => {
                *x_pos = *f;
                Ok(())
            }
            interaction => Err(format!(
                "Got unexpected value type: '{:?}' for an interaction '{:?}'",
                value, interaction
            )),
        }
    }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct InteractionId(usize);

impl InteractionId {
    /// Get ImGui unique ID
    pub fn id(self) -> i32 {
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

#[derive(Debug)]
pub struct FilterRoi<'a>(InteractionIterMut<'a>);

impl<'a> InteractionIterMut<'a> {
    pub fn filter_roi(self) -> FilterRoi<'a> {
        FilterRoi(self)
    }
}

impl<'a> Iterator for FilterRoi<'a> {
    type Item = (&'a InteractionId, &'a mut Interaction);
    fn next(&mut self) -> Option<Self::Item> {
        for x in &mut self.0 {
            if let (_, Interaction::FinedGrainedROI(_)) = x {
                return Some(x);
            }
        }
        None
    }
}
