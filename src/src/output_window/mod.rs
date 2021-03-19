use std::collections::HashMap;
use std::error;

mod menu_bar;
mod visualizable;

use glium;
use imgui::{Ui, Window};
use owning_ref::ArcRef;

use crate::cake::{OutputId, TransformIdx};
use crate::primitives::{ndarray, IOValue, SuccessOut};
use aflak_plot::{
    imshow::{self, Textures},
    plot, InteractionId,
};

use self::menu_bar::MenuBar;
use self::visualizable::{Initializing, Unimplemented, Visualizable};
use crate::aflak::AflakNodeEditor;

#[derive(Default)]
pub struct OutputWindow {
    image1d_state: plot::State,
    image2d_state: imshow::State<ArcRef<IOValue, ndarray::ArrayD<f32>>>,
    editable_values: EditableValues,
    show_pixels: bool,
}

type EditableValues = HashMap<InteractionId, TransformIdx>;

impl OutputWindow {
    pub fn draw<'ui, F>(
        &mut self,
        ui: &'ui Ui,
        output: OutputId,
        window: Window<'_>,
        node_editor: &mut AflakNodeEditor,
        gl_ctx: &F,
        textures: &mut Textures,
    ) -> Vec<Box<dyn error::Error>>
    where
        F: glium::backend::Facade,
    {
        let compute_state = node_editor.compute_output(output);
        match compute_state {
            None => {
                Initializing.draw(ui, window);
                vec![]
            }
            Some(Err(e)) => {
                e.draw(ui, window);
                vec![]
            }
            Some(Ok(result)) => {
                let created_on = SuccessOut::created_on(&result);
                let value = SuccessOut::take(result);
                let ctx = menu_bar::OutputWindowCtx {
                    ui,
                    output,
                    value: &value,
                    window: self,
                    created_on,
                    node_editor,
                    gl_ctx,
                    textures,
                };
                match &*value {
                    IOValue::Str(ref string) => string.draw(ctx, window),
                    IOValue::Integer(integer) => integer.draw(ctx, window),
                    IOValue::Float(float) => float.draw(ctx, window),
                    IOValue::Float2(floats) => floats.draw(ctx, window),
                    IOValue::Float3(floats) => floats.draw(ctx, window),
                    IOValue::Bool(b) => b.draw(ctx, window),
                    IOValue::Image(ref image) => image.draw(ctx, window),
                    IOValue::Roi(ref roi) => roi.draw(ctx, window),
                    IOValue::Fits(ref fits) => {
                        fits.draw(ui, window);
                        vec![]
                    }
                    val => {
                        Unimplemented::new(val).draw(ui, window);
                        vec![]
                    }
                }
            }
        }
    }
}
