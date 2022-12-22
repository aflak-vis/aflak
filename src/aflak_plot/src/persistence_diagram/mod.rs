//! Draw scatter_plot.
mod state;

use aflak_primitives::PersistencePairs;
use imgui::Ui;
use implot::PlotUi;
use std::collections::HashMap;
use std::time::Instant;

pub use self::interactions::InteractionId;
use super::interactions;
use super::Error;
use crate::imshow::cake::{OutputId, TransformIdx};
type EditabaleValues = HashMap<InteractionId, TransformIdx>;
pub use self::state::State;

/// Implementation of a UI to visualize a 1D image with ImGui using a plot.
pub trait UiPersistenceDiagram {
    fn persistence_diagram(
        &self,
        data: &PersistencePairs,
        plot_ui: &PlotUi,
        state: &mut State,
        copying: &mut Option<(InteractionId, TransformIdx)>,
        store: &mut EditabaleValues,
        attaching: &mut Option<(OutputId, TransformIdx, usize)>,
        created_on: Instant,
        outputid: OutputId,
    ) -> Result<(), Error>;
}

impl<'ui> UiPersistenceDiagram for Ui<'ui> {
    /// Draw a plot in the remaining space of the window.
    ///
    /// The mutable reference `state` contains the current state of the user
    /// interaction with the window.
    fn persistence_diagram(
        &self,
        data: &PersistencePairs,
        plot_ui: &PlotUi,
        state: &mut State,
        copying: &mut Option<(InteractionId, TransformIdx)>,
        store: &mut EditabaleValues,
        attaching: &mut Option<(OutputId, TransformIdx, usize)>,
        created_on: Instant,
        outputid: OutputId,
    ) -> Result<(), Error> {
        let p = self.cursor_screen_pos();
        let window_pos = self.window_pos();
        let window_size = self.window_size();
        let _size = [window_size[0], window_size[1] - (p[1] - window_pos[1])];
        state.persistence_diagram(
            self,
            data,
            &plot_ui,
            &mut *copying,
            &mut *store,
            &mut *attaching,
            created_on,
            outputid,
        )
    }
}
