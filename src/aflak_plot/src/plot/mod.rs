//! Draw plots.
mod state;

use imgui::Ui;
use ndarray::{ArrayBase, Data, Ix1};
use std::collections::HashMap;

extern crate aflak_cake as cake;
pub extern crate aflak_primitives as primitives;
pub extern crate node_editor;
use super::interactions;
use super::lims;
use super::ticks;
use super::util;
use super::AxisTransform;
use super::Error;

pub use self::cake::TransformIdx;
pub use self::interactions::InteractionId;
pub use self::state::State;
use super::node_editor::NodeEditor;
use super::primitives::{IOErr, IOValue};

type EditableValues = HashMap<InteractionId, TransformIdx>;
type AflakNodeEditor = NodeEditor<IOValue, IOErr>;

/// Implementation of a UI to visualize a 1D image with ImGui using a plot.
pub trait UiImage1d {
    fn image1d<S, F>(
        &self,
        image: &ArrayBase<S, Ix1>,
        vtype: &str,
        vunit: &str,
        axis: Option<&AxisTransform<F>>,
        state: &mut State,
        copying: &mut Option<(InteractionId, TransformIdx)>,
        store: &mut EditableValues,
        attaching: &mut Option<(cake::OutputId, TransformIdx, usize)>,
        outputid: cake::OutputId,
        node_editor: &AflakNodeEditor,
    ) -> Result<(), Error>
    where
        S: Data<Elem = f32>,
        F: Fn(f32) -> f32;
}

impl<'ui> UiImage1d for Ui<'ui> {
    /// Draw a plot in the remaining space of the window.
    ///
    /// The mutable reference `state` contains the current state of the user
    /// interaction with the window.
    fn image1d<S, F>(
        &self,
        image: &ArrayBase<S, Ix1>,
        vtype: &str,
        vunit: &str,
        axis: Option<&AxisTransform<F>>,
        state: &mut State,
        copying: &mut Option<(InteractionId, TransformIdx)>,
        store: &mut EditableValues,
        attaching: &mut Option<(cake::OutputId, TransformIdx, usize)>,
        outputid: cake::OutputId,
        node_editor: &AflakNodeEditor,
    ) -> Result<(), Error>
    where
        S: Data<Elem = f32>,
        F: Fn(f32) -> f32,
    {
        let p = self.cursor_screen_pos();
        let window_pos = self.window_pos();
        let window_size = self.window_size();
        let size = [window_size[0], window_size[1] - (p[1] - window_pos[1])];
        state.plot(
            self,
            image,
            vtype,
            vunit,
            axis,
            p,
            size,
            copying,
            store,
            attaching,
            outputid,
            node_editor,
        )
    }
}
