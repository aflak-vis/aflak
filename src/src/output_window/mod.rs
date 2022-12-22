use std::collections::HashMap;
use std::error;

mod menu_bar;
mod visualizable;

use glium;
use imgui::{Ui, Window};
use owning_ref::ArcRef;

use crate::aflak_plot::{
    imshow::{self, Textures},
    persistence_diagram, plot, plot_colormap, scatter_lineplot, InteractionId,
};
use crate::cake::{OutputId, TransformIdx};
use crate::primitives::{ndarray, IOValue, SuccessOut};
use implot::Context;

use self::menu_bar::MenuBar;
use self::visualizable::{Initializing, Unimplemented, Visualizable};
use crate::aflak::AflakNodeEditor;

#[derive(Default)]
pub struct OutputWindow {
    image1d_state: plot::State,
    image2d_state: imshow::State<ArcRef<IOValue, ndarray::ArrayD<f32>>>,
    colormap_state: plot_colormap::State,
    scatter_lineplot_state: scatter_lineplot::State,
    persistence_diagram_state: persistence_diagram::State,
    pub editable_values: EditableValues,
    show_pixels: bool,
}

type EditableValues = HashMap<InteractionId, TransformIdx>;

impl OutputWindow {
    pub fn draw<'ui, F, T>(
        &mut self,
        ui: &'ui Ui,
        output: OutputId,
        window: Window<'_, T>,
        node_editor: &mut AflakNodeEditor,
        gl_ctx: &F,
        textures: &mut Textures,
        plotcontext: &Context,
        copying: &mut Option<(InteractionId, TransformIdx)>,
        attaching: &mut Option<(OutputId, TransformIdx, usize)>,
    ) -> Vec<Box<dyn error::Error>>
    where
        F: glium::backend::Facade,
        T: AsRef<str>,
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
                    plotcontext,
                    copying,
                    attaching,
                };
                match &*value {
                    IOValue::Str(ref string) => string.draw(ctx, window),
                    IOValue::Paths(ref files) => files.draw(ctx, window),
                    IOValue::Integer(integer) => integer.draw(ctx, window),
                    IOValue::Float(float) => float.draw(ctx, window),
                    IOValue::Float2(floats) => floats.draw(ctx, window),
                    IOValue::Float3(floats) => floats.draw(ctx, window),
                    IOValue::Float3x3(floats) => floats.draw(ctx, window),
                    IOValue::Bool(b) => b.draw(ctx, window),
                    IOValue::Image(ref image) => image.draw(ctx, window),
                    IOValue::Roi(ref roi) => roi.draw(ctx, window),
                    IOValue::ColorLut(ref colorlut) => colorlut.draw(ctx, window),
                    IOValue::Fits(ref fits) => {
                        fits.draw(ui, window);
                        vec![]
                    }
                    IOValue::PersistencePairs(ref pp) => pp.draw(ctx, window),
                    val => {
                        Unimplemented::new(val).draw(ui, window);
                        vec![]
                    }
                }
            }
        }
    }
}
