extern crate aflak_cake as cake;
extern crate glium;

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::collections::btree_set;
use cake::{TransformIdx, DST};
use glium::{DrawError, Surface};

/// Draw options to be provided to the draw function
pub struct DrawOptions {
    pub clear_color: [f32; 4],
}

impl Default for DrawOptions {
    fn default() -> Self {
        Self {
            clear_color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

/// Representation to draw DST
pub struct Diagram<'t, T: 't + Clone, E: 't> {
    dst: &'t DST<'t, T, E>,
    boxes: BTreeSet<DiagramBox>,
}

#[derive(Debug, PartialOrd, PartialEq)]
struct DiagramBox {
    z_depth: u32,
    position: [f32; 2],
    t_idx: TransformIdx,
}

impl Eq for DiagramBox {}

impl Ord for DiagramBox {
    fn cmp(&self, other: &Self) -> Ordering {
        self.z_depth
            .cmp(&other.z_depth)
            .then_with(|| {
                self.position[0]
                    .partial_cmp(&other.position[0])
                    .unwrap_or(Ordering::Less)
            })
            .then_with(|| {
                self.position[1]
                    .partial_cmp(&other.position[1])
                    .unwrap_or(Ordering::Less)
            })
            .then_with(|| self.t_idx.cmp(&other.t_idx))
    }
}

impl<'t, T: Clone, E> Diagram<'t, T, E> {
    pub fn new(dst: &'t DST<'t, T, E>) -> Self {
        let mut boxes = BTreeSet::new();
        let mut px = 0.0;
        for (&t_idx, _t) in dst.transforms_iter() {
            boxes.insert(
                // TODO: Make an intelligent layout
                DiagramBox {
                    z_depth: 0,
                    position: [px, 0.0],
                    t_idx,
                },
            );
            px += 0.1;
        }
        Self { dst, boxes }
    }

    fn box_iter(&self) -> btree_set::Iter<DiagramBox> {
        self.boxes.iter()
    }
}

/// Draw the DST to the given target
pub fn draw<'t, S, T, E>(
    target: &mut S,
    dst: &Diagram<'t, T, E>,
    options: &DrawOptions,
) -> Result<(), DrawError>
where
    S: Surface,
    T: Clone,
{
    let [r, g, b, a] = options.clear_color;
    target.clear_color(r, g, b, a);

    for d_box in dst.box_iter() {
        println!("{:?}", d_box);
    }

    Ok(())
}
