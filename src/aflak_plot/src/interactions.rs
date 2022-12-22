use std::collections::{btree_map, BTreeMap};

/// A value of an interaction with the UI.
///
/// For example, a vertical line would have a value corresponding to its
/// *x*-coordinates with the variant `Value::Integer`.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f32),
    Float2([f32; 2]),
    Float3([f32; 3]),
    Float3x3([[f32; 3]; 3]),
    FinedGrainedROI((Vec<(usize, usize)>, bool)),
    Line(Vec<(usize, usize)>),
    Circle(Vec<(usize, usize)>),
    ColorLut((usize, Vec<(f32, [u8; 3])>)),
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
impl From<[[f32; 3]; 3]> for Value {
    fn from(v: [[f32; 3]; 3]) -> Self {
        Value::Float3x3(v)
    }
}
impl From<(Vec<(usize, usize)>, bool)> for Value {
    fn from(v: (Vec<(usize, usize)>, bool)) -> Self {
        Value::FinedGrainedROI(v)
    }
}
impl From<(usize, Vec<(f32, [u8; 3])>)> for Value {
    fn from(v: (usize, Vec<(f32, [u8; 3])>)) -> Self {
        Value::ColorLut(v)
    }
}

/// Possible interactions with UI.
#[derive(Clone, Debug, PartialEq)]
pub enum Interaction {
    HorizontalLine(HorizontalLine),
    VerticalLine(VerticalLine),
    FinedGrainedROI(FinedGrainedROI),
    Line(Line),
    Circle(Circle),
    Lims(Lims),
    ColorLims(ColorLims),
    ColorLut(ColorLut),
    PersistenceFilter(PersistenceFilter),
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
    pub changed: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub endpoints: ((f32, f32), (f32, f32)),
    pub endpoints_zero: ((f32, f32), (f32, f32)),
    pub endpointsfill: (bool, bool),
    pub pixels: Vec<(usize, usize)>,
    pub pre_mousepos: (f32, f32),
    pub allmoving: bool,
    pub edgemoving: (bool, bool),
    pub show_rotate: bool,
    pub degree: i32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Circle {
    pub(crate) id: usize,
    pub center: (usize, usize),
    pub radius: f32,
    pub parametersfill: (bool, bool),
    pub pixels: Vec<(usize, usize)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Lims {
    pub lims: [f32; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct ColorLims {
    pub lims: [[f32; 3]; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct ColorLut {
    pub colormode: usize,
    pub lut: Vec<(f32, [u8; 3])>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PersistenceFilter {
    pub val: f32,
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
        Self {
            id,
            pixels: vec![],
            changed: false,
        }
    }

    pub fn from_vec(id: usize, vec: Vec<(usize, usize)>) -> Self {
        Self {
            id,
            pixels: vec,
            changed: false,
        }
    }
}

impl ColorLut {
    pub fn new(colormode: usize, lut: Vec<(f32, [u8; 3])>) -> Self {
        Self { colormode, lut }
    }
}

impl Line {
    pub fn new() -> Self {
        Self {
            endpoints: ((0.0, 0.0), (0.0, 0.0)),
            endpoints_zero: ((0.0, 0.0), (0.0, 0.0)),
            endpointsfill: (false, false),
            pixels: vec![],
            pre_mousepos: (0.0, 0.0),
            allmoving: false,
            edgemoving: (false, false),
            show_rotate: false,
            degree: 0,
        }
    }
}

impl Circle {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            center: (0, 0),
            radius: 0.0,
            parametersfill: (false, false),
            pixels: vec![],
        }
    }
}

impl Lims {
    pub fn new(lims: [f32; 3]) -> Self {
        Self { lims }
    }
}

impl ColorLims {
    pub fn new(lims: [[f32; 3]; 3]) -> Self {
        Self { lims }
    }
}

impl PersistenceFilter {
    pub fn new(val: f32) -> Self {
        Self { val, moving: false }
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

    pub fn id(&mut self) -> InteractionId {
        InteractionId(self.1)
    }

    pub(crate) fn remove(&mut self, id: InteractionId) -> Option<Interaction> {
        self.0.remove(&id)
    }

    pub(crate) fn any_moving(&self) -> bool {
        self.0.iter().any(|(_, interaction)| match interaction {
            Interaction::HorizontalLine(HorizontalLine { moving, .. }) => *moving,
            Interaction::VerticalLine(VerticalLine { moving, .. }) => *moving,
            Interaction::FinedGrainedROI(..) => false,
            Interaction::Line(..) => false,
            Interaction::Circle(..) => false,
            Interaction::Lims(..) => false,
            Interaction::ColorLims(..) => false,
            Interaction::ColorLut(..) => false,
            Interaction::PersistenceFilter(PersistenceFilter { moving, .. }) => *moving,
        })
    }
}

impl Interaction {
    pub(crate) fn value(&self) -> Value {
        match self {
            Interaction::HorizontalLine(HorizontalLine { height, .. }) => Value::Float(*height),
            Interaction::VerticalLine(VerticalLine { x_pos, .. }) => Value::Float(*x_pos),
            Interaction::FinedGrainedROI(FinedGrainedROI {
                id: _,
                pixels,
                changed,
            }) => Value::FinedGrainedROI((pixels.clone(), *changed)),
            Interaction::Line(Line { pixels, .. }) => Value::Line(pixels.clone()),
            Interaction::Circle(Circle { pixels, .. }) => Value::Circle(pixels.clone()),
            Interaction::Lims(Lims { lims, .. }) => Value::Float3(*lims),
            Interaction::ColorLims(ColorLims { lims, .. }) => Value::Float3x3(*lims),
            Interaction::ColorLut(ColorLut { colormode, lut }) => {
                Value::ColorLut((*colormode, lut.clone()))
            }
            Interaction::PersistenceFilter(PersistenceFilter { val, .. }) => Value::Float(*val),
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
            (
                Interaction::FinedGrainedROI(FinedGrainedROI {
                    id: _,
                    ref mut pixels,
                    ref mut changed,
                }),
                Value::FinedGrainedROI(p),
            ) => {
                if *pixels != (*p).0.clone() {
                    *changed = true;
                } else {
                    *changed = false;
                }
                *pixels = (*p).0.clone();
                Ok(())
            }
            (Interaction::Lims(Lims { ref mut lims, .. }), Value::Float3(f3)) => {
                for i in 0..3 {
                    lims[i] = Interaction::clamp(f3[i], 0.0, 1.0);
                }
                Ok(())
            }
            (Interaction::ColorLims(ColorLims { ref mut lims, .. }), Value::Float3x3(f3)) => {
                for c in 0..3 {
                    for v in 0..3 {
                        lims[c][v] = Interaction::clamp(f3[c][v], 0.0, 1.0);
                    }
                }
                Ok(())
            }
            (
                Interaction::ColorLut(ColorLut {
                    ref mut colormode,
                    ref mut lut,
                }),
                Value::ColorLut(l),
            ) => {
                *colormode = l.0;
                *lut = l.1.clone();
                Ok(())
            }
            (
                Interaction::PersistenceFilter(PersistenceFilter { ref mut val, .. }),
                Value::Float(f),
            ) => {
                *val = *f;
                Ok(())
            }
            interaction => Err(format!(
                "Got unexpected value type: '{:?}' for an interaction '{:?}'",
                value, interaction
            )),
        }
    }

    fn clamp<T>(v: T, min: T, max: T) -> T
    where
        T: PartialOrd,
    {
        if v < min {
            min
        } else if v > max {
            max
        } else {
            v
        }
    }
}

/// ID identifying an interaction on a UI.
///
/// For example, drawing a vertical line or selecting a ROI are types of
/// interaction.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct InteractionId(usize);

impl InteractionId {
    /// Get ImGui unique ID
    pub fn id(self) -> i32 {
        self.0 as i32
    }
}

/// Iterator over of the current values of each of interactions of the UI.
#[derive(Clone, Debug)]
pub struct ValueIter<'a>(btree_map::Iter<'a, InteractionId, Interaction>);

/// Iterator over the `Interaction`s of a UI.
#[derive(Debug)]
pub struct InteractionIterMut<'a>(btree_map::IterMut<'a, InteractionId, Interaction>);

impl<'a> Iterator for ValueIter<'a> {
    type Item = (&'a InteractionId, &'a Interaction, Value);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(id, inter)| (id, inter, inter.value()))
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
