use imgui::{ImVec2, Ui};
use ndarray::Array1;

use super::lims;
use super::ticks;
use super::Error;

#[derive(Clone, Debug)]
pub struct State {
    offset: (f32, f32),
}

impl Default for State {
    fn default() -> Self {
        State { offset: (0.0, 0.0) }
    }
}

impl State {
    pub(crate) fn plot<P, S>(
        &mut self,
        ui: &Ui,
        image: &Array1<f32>,
        pos: P,
        size: S,
    ) -> Result<(), Error>
    where
        P: Into<ImVec2>,
        S: Into<ImVec2>,
    {
        ui.set_cursor_screen_pos(pos);
        let p = ui.get_cursor_screen_pos();
        let min = lims::get_vmin(image)?;
        let max = lims::get_vmax(image)?;
        ui.plot_lines(im_str!(""), image.view().into_slice().expect("Get slice"))
            .graph_size(size)
            .scale_min(min)
            .scale_max(max)
            .build();
        let size = ui.get_item_rect_size();
        let draw_list = ui.get_window_draw_list();
        ticks::add_ticks(
            ui,
            &draw_list,
            p,
            size,
            (0.0, image.len() as f32),
            (min, max),
        );

        ui.text(format!("{} data points", image.len()));

        Ok(())
    }
}
