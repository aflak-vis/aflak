use imgui::{ImGuiMouseCursor, ImMouseButton, ImVec2, Ui};
use ndarray::Array1;

use super::lims;
use super::ticks;
use super::Error;

#[derive(Clone, Debug)]
pub struct State {
    offset: ImVec2,
    zoom: ImVec2,
}

impl Default for State {
    fn default() -> Self {
        State {
            offset: ImVec2 { x: 0.0, y: 0.0 },
            zoom: ImVec2 { x: 1.0, y: 1.0 },
        }
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

        let xvlims = (0.0, (image.len() - 1) as f32);
        let yvlims = (min, max);
        let xlims = (
            xvlims.0 * self.zoom.x + self.offset.x,
            xvlims.1 * self.zoom.x + self.offset.x,
        );
        let ylims = (
            yvlims.0 * self.zoom.y + self.offset.y,
            yvlims.1 * self.zoom.y + self.offset.y,
        );

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

            // Zoom along X-axis
            let wheel_delta = ui.imgui().mouse_wheel();

            if wheel_delta != 0.0 {
                const ZOOM_SPEED: f32 = 0.5;

                let zoom_before = self.zoom.x;
                self.zoom.x *= 1.0 - wheel_delta * ZOOM_SPEED;
                // Correct offset value so that the zoom be centered on the mouse position
                self.offset.x -= (self.zoom.x - zoom_before)
                    * (xvlims.0 + (mouse_x - p.0) / size.x * (xvlims.1 - xvlims.0));
            }

            // Scroll using mouse wheel
            if ui.imgui().is_mouse_dragging(ImMouseButton::Left) {
                ui.imgui().set_mouse_cursor(ImGuiMouseCursor::Move);
                let delta = ui.imgui().mouse_delta();
                self.offset.x -= delta.0 / size.x * (xlims.1 - xlims.0);
                self.offset.y += delta.1 / size.y * (ylims.1 - ylims.0);
            }
        }

        ticks::add_ticks(ui, &draw_list, p, size, xlims, ylims);

        ui.text(format!("{} data points", image.len()));

        Ok(())
    }
}
