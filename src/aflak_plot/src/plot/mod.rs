use imgui::Ui;
use ndarray::Array1;

use super::lims;
use super::ticks;
use super::Error;

pub trait UiImage1d {
    fn image1d(&self, image: &Array1<f32>, state: &mut State) -> Result<(), Error>;
}

impl<'ui> UiImage1d for Ui<'ui> {
    fn image1d(&self, image: &Array1<f32>, _: &mut State) -> Result<(), Error> {
        const PLOT_HEIGHT: f32 = 400.0;
        const PLOT_LEFT_PADDING: f32 = 20.0;
        let p = self.get_cursor_screen_pos();
        self.set_cursor_screen_pos([p.0 + PLOT_LEFT_PADDING, p.1]);
        let p = self.get_cursor_screen_pos();
        let min = lims::get_vmin(image)?;
        let max = lims::get_vmax(image)?;
        self.plot_lines(im_str!(""), image.view().into_slice().expect("Get slice"))
            .graph_size([0.0, PLOT_HEIGHT])
            .scale_min(min)
            .scale_max(max)
            .build();
        let size = self.get_item_rect_size();
        let draw_list = self.get_window_draw_list();
        ticks::add_ticks(
            self,
            &draw_list,
            p,
            size,
            (0.0, image.len() as f32),
            (min, max),
        );

        self.text(format!("{} data points", image.len()));

        Ok(())
    }
}

pub struct State;

impl Default for State {
    fn default() -> Self {
        State
    }
}
