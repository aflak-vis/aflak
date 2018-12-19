use imgui::{ImGuiMouseCursor, ImMouseButton, ImVec2, Ui};
use ndarray::{ArrayBase, Data, Ix1};

use super::interactions::{
    Interaction, InteractionIterMut, Interactions, ValueIter, VerticalLine, VerticalLineEvent,
};
use super::lims;
use super::ticks::XYTicks;
use super::AxisTransform;
use super::Error;

#[derive(Debug)]
pub struct State {
    offset: ImVec2,
    zoom: ImVec2,
    mouse_pos: ImVec2,
    interactions: Interactions,
}

impl Default for State {
    fn default() -> Self {
        use std::f32;
        State {
            offset: ImVec2 { x: 0.0, y: 0.0 },
            zoom: ImVec2 { x: 1.0, y: 1.0 },
            mouse_pos: ImVec2 {
                x: f32::NAN,
                y: f32::NAN,
            },
            interactions: Interactions::new(),
        }
    }
}

impl State {
    pub fn stored_values(&self) -> ValueIter {
        self.interactions.value_iter()
    }

    pub fn stored_values_mut(&mut self) -> InteractionIterMut {
        self.interactions.iter_mut()
    }

    pub(crate) fn plot<D, F, P, S>(
        &mut self,
        ui: &Ui,
        image: &ArrayBase<D, Ix1>,
        vunit: &str,
        axis: Option<&AxisTransform<F>>,
        pos: P,
        size: S,
    ) -> Result<(), Error>
    where
        D: Data<Elem = f32>,
        F: Fn(f32) -> f32,
        P: Into<ImVec2>,
        S: Into<ImVec2>,
    {
        let pos = pos.into();
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

        // Pre-compute tick size to accurately position and resize the figure
        // to fit everything in the "size" given as input to this function.
        let yaxis = AxisTransform::id(vunit);
        let ticks = XYTicks::prepare(ui, xlims, ylims, axis, Some(&yaxis));
        let x_labels_height = ticks.x_labels_height();
        let y_labels_width = ticks.y_labels_width();

        const BOTTOM_PADDING: f32 = 40.0;
        const RIGHT_PADDING: f32 = 20.0;
        let size = ImVec2 {
            x: size.x - y_labels_width - RIGHT_PADDING,
            y: size.y - x_labels_height - BOTTOM_PADDING,
        };

        // Start drawing the figure
        ui.set_cursor_screen_pos([pos.x + y_labels_width, pos.y]);
        let p = ui.get_cursor_screen_pos();

        ui.invisible_button(im_str!("plot"), size);

        let draw_list = ui.get_window_draw_list();

        const BG_COLOR: u32 = 0xA033_3333;
        const LINE_COLOR: u32 = 0xFFFF_FFFF;

        let bottom_right_corner = [p.0 + size.x, p.1 + size.y];

        draw_list.with_clip_rect_intersect(p, bottom_right_corner, || {
            draw_list
                .add_rect(p, bottom_right_corner, BG_COLOR)
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
            self.mouse_pos.x = xlims.0 + (mouse_x - p.0) / size.x * (xlims.1 - xlims.0);
            let point = self.mouse_pos.x as usize;
            if let Some(y) = image.get(point) {
                let x = axis.map(|axis| Measurement {
                    v: axis.pix2world(self.mouse_pos.x),
                    unit: axis.unit(),
                });
                let val = Measurement { v: *y, unit: vunit };
                let text = self.make_tooltip(point, x, val);
                ui.tooltip_text(text);
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

            // Pan by dragging mouse
            if !self.interactions.any_moving() && ui.imgui().is_mouse_dragging(ImMouseButton::Left)
            {
                ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeAll);
                let delta = ui.imgui().mouse_delta();
                self.offset.x -= delta.0 / size.x * (xlims.1 - xlims.0);
                self.offset.y += delta.1 / size.y * (ylims.1 - ylims.0);
            }

            if ui.imgui().is_mouse_clicked(ImMouseButton::Right) {
                ui.open_popup(im_str!("add-interaction-handle"))
            }
        }

        let mut line_marked_for_deletion = None;
        // Flag to only allow line to be moved at a time
        for (id, interaction) in self.interactions.iter_mut() {
            match interaction {
                Interaction::VerticalLine(line) => {
                    let event = line.draw(ui, &draw_list, *id, p, size, xlims);
                    if let Some(VerticalLineEvent::Delete) = event {
                        line_marked_for_deletion = Some(*id);
                    }
                }
                // Unused in plot
                Interaction::HorizontalLine(_) => {}
                Interaction::FinedGrainedROI(_) => {}
            }
        }

        if let Some(line_id) = line_marked_for_deletion {
            self.interactions.remove(line_id);
        }

        ticks.draw(&draw_list, p, size);

        // Add interaction handlers
        ui.popup(im_str!("add-interaction-handle"), || {
            ui.text("Add interaction handle");
            ui.separator();
            if ui.menu_item(im_str!("Vertical Line")).build() {
                let new = Interaction::VerticalLine(VerticalLine::new(self.mouse_pos.x.round()));
                self.interactions.insert(new);
            }
            ui.separator();
            if ui.menu_item(im_str!("Reset view")).build() {
                self.zoom = ImVec2 { x: 1.0, y: 1.0 };
                self.offset = ImVec2 { x: 0.0, y: 0.0 };
            }
        });

        Ok(())
    }

    fn make_tooltip(&self, point: usize, x: Option<Measurement>, y: Measurement) -> String {
        let x_str = if let Some(x) = x {
            if x.unit.is_empty() {
                format!("X:   {:.2}", x.v)
            } else {
                format!("X:   {:.2} {}", x.v, x.unit)
            }
        } else {
            format!("X:    {}", point)
        };

        let val = if y.unit.is_empty() {
            format!("VAL: {:.2}", y.v)
        } else {
            format!("VAL: {:.2} {}", y.v, y.unit)
        };

        if x.is_some() {
            format!("{} (at point {})\n{}", x_str, point, val)
        } else {
            format!("{}\n{}", x_str, val)
        }
    }
}

#[derive(Copy, Clone)]
pub struct Measurement<'a> {
    pub v: f32,
    pub unit: &'a str,
}
