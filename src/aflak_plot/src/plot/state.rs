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
        let size = size.into();

        let min = lims::get_vmin(image)?;
        let max = lims::get_vmax(image)?;

        let xlims = (0.0, (image.len() - 1) as f32);
        let ylims = (min, max);

        ui.set_cursor_screen_pos(pos);
        let p = ui.get_cursor_screen_pos();

        ui.invisible_button(im_str!("plot"), size);

        let draw_list = ui.get_window_draw_list();

        const BG_COLOR: u32 = 0xA0333333;
        const LINE_COLOR: u32 = 0xFFFFFFFF;

        let bottom_right_corner = [p.0 + size.x, p.1 + size.y];

        draw_list.with_clip_rect_intersect(p, bottom_right_corner, || {
            draw_list
                .add_rect(p, [p.0 + size.x, p.1 + size.y], BG_COLOR)
                .filled(true)
                .build();

            let first = image.iter().enumerate();
            let second = image.iter().enumerate().skip(1);
            for ((x1, y1), (x2, y2)) in first.zip(second) {
                let x1 = x1 as f32;
                let x2 = x2 as f32;
                let p0 = [
                    p.0 + (x1 - xlims.0) / (xlims.1 - xlims.0) * size.x,
                    p.1 + size.y - (y1 - ylims.0) / (ylims.1 - ylims.0) * size.y,
                ];
                let p1 = [
                    p.0 + (x2 - xlims.0) / (xlims.1 - xlims.0) * size.x,
                    p.1 + size.y - (y2 - ylims.0) / (ylims.1 - ylims.0) * size.y,
                ];
                draw_list.add_line(p0, p1, LINE_COLOR).build();
            }
        });

        if ui.is_item_hovered() {
            let mouse_x = ui.imgui().mouse_pos().0;
            let x = xlims.0 + (mouse_x - p.0) / size.x * (xlims.1 - xlims.0);
            if let Some(y) = image.get(x as usize) {
                ui.text(format!("X: {:.0}, VAL: {:.2}", x, y));
            }
        }

        ticks::add_ticks(ui, &draw_list, p, size, xlims, ylims);

        ui.text(format!("{} data points", image.len()));

        Ok(())
    }
}
