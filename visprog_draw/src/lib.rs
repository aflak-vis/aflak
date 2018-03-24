extern crate aflak_cake as cake;
#[macro_use]
extern crate glium;

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::collections::btree_set;
use cake::{TransformIdx, DST};
use glium::{DrawError, Program, Surface};
use glium::backend::Facade;

/// Draw options to be provided to the draw function
pub struct DrawOptions {
    pub clear_color: [f32; 4],
    pub box_width: f32,
    pub box_height: f32,
}

impl Default for DrawOptions {
    fn default() -> Self {
        Self {
            clear_color: [1.0, 1.0, 1.0, 1.0],
            box_width: 0.085,
            box_height: 0.085,
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
        let mut px = -0.95;
        for (&t_idx, _t) in dst.transforms_iter() {
            boxes.insert(
                // TODO: Make an intelligent layout
                DiagramBox {
                    z_depth: 0,
                    position: [px, 0.95],
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

pub struct DrawContext<'a, F: 'a> {
    facade: &'a F,
    box_program: Program,
}

pub fn get_context<F>(facade: &F) -> DrawContext<F>
where
    F: Facade,
{
    let vertex_shader_src = r#"
        #version 140

        in vec2 position;

        void main() {
            gl_Position = vec4(position, 0.0, 1.0);
        }
    "#;
    let fragment_shader_src = r#"
        #version 140

        out vec4 color;

        void main() {
            color = vec4(0.0, 0.0, 0.0, 1.0);
        }
    "#;
    DrawContext {
        facade,
        box_program: Program::from_source(facade, vertex_shader_src, fragment_shader_src, None)
            .expect("Correct program"),
    }
}

/// Draw the DST to the given target
pub fn draw<'t, S, F, T, E>(
    target: &mut S,
    diag: &Diagram<'t, T, E>,
    ctx: &DrawContext<F>,
    options: &DrawOptions,
) -> Result<(), DrawError>
where
    S: Surface,
    F: Facade,
    T: Clone,
{
    let [r, g, b, a] = options.clear_color;
    target.clear_color(r, g, b, a);

    for d_box in diag.box_iter() {
        draw_box(target, d_box, ctx, options)?;
    }

    Ok(())
}

fn draw_box<S, F>(
    target: &mut S,
    d_box: &DiagramBox,
    ctx: &DrawContext<F>,
    options: &DrawOptions,
) -> Result<(), DrawError>
where
    S: Surface,
    F: Facade,
{
    let vertex1 = Vertex {
        position: d_box.position,
    };
    let vertex2 = Vertex {
        position: [d_box.position[0], d_box.position[1] - options.box_height],
    };
    let vertex3 = Vertex {
        position: [
            d_box.position[0] + options.box_width,
            d_box.position[1] - options.box_height,
        ],
    };
    let vertex4 = Vertex {
        position: [d_box.position[0] + options.box_width, d_box.position[1]],
    };
    let shape = vec![vertex1, vertex2, vertex3, vertex4];
    let vertex_buffer = glium::VertexBuffer::new(ctx.facade, &shape).unwrap();
    let index_buffer = glium::index::NoIndices(glium::index::PrimitiveType::LineLoop);
    target.draw(
        &vertex_buffer,
        &index_buffer,
        &ctx.box_program,
        &glium::uniforms::EmptyUniforms,
        &Default::default(),
    )
}

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}

implement_vertex!(Vertex, position);
