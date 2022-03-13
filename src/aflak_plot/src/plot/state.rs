use imgui::{MenuItem, MouseButton, MouseCursor, Ui};
use ndarray::{ArrayBase, Data, Ix1};
use std::collections::HashMap;

use super::interactions::{
    Interaction, InteractionId, InteractionIterMut, Interactions, ValueIter, VerticalLine,
};
use super::lims;
use super::node_editor::NodeEditor;
use super::primitives::{IOErr, IOValue};
use super::ticks::XYTicks;
use super::util;
use super::AxisTransform;
use super::Error;

use crate::plot::cake::{OutputId, Transform, TransformIdx};

type EditableValues = HashMap<InteractionId, TransformIdx>;
type AflakNodeEditor = NodeEditor<IOValue, IOErr>;

/// Current state of a plot UI.
#[derive(Debug)]
pub struct State {
    offset: [f32; 2],
    zoom: [f32; 2],
    mouse_pos: [f32; 2],
    interactions: Interactions,
    pub show_axis_option: bool,
}

impl Default for State {
    fn default() -> Self {
        use std::f32;
        State {
            offset: [0.0, 0.0],
            zoom: [1.0, 1.0],
            mouse_pos: [f32::NAN, f32::NAN],
            interactions: Interactions::new(),
            show_axis_option: false,
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

    pub(crate) fn plot<D, F>(
        &mut self,
        ui: &Ui,
        image: &ArrayBase<D, Ix1>,
        vtype: &str,
        vunit: &str,
        axis: Option<&AxisTransform<F>>,
        pos: [f32; 2],
        size: [f32; 2],
        copying: &mut Option<(InteractionId, TransformIdx)>,
        store: &mut EditableValues,
        attaching: &mut Option<(OutputId, TransformIdx, usize)>,
        outputid: OutputId,
        node_editor: &AflakNodeEditor,
    ) -> Result<(), Error>
    where
        D: Data<Elem = f32>,
        F: Fn(f32) -> f32,
    {
        let min = lims::get_vmin(image)?;
        let max = lims::get_vmax(image)?;

        let xvlims = (0.0, (image.len() - 1) as f32);
        let yvlims = (min, max);
        let xlims = (
            xvlims.0 * self.zoom[0] + self.offset[0],
            xvlims.1 * self.zoom[0] + self.offset[0],
        );
        let ylims = (
            yvlims.0 * self.zoom[1] + self.offset[1],
            yvlims.1 * self.zoom[1] + self.offset[1],
        );

        // Pre-compute tick size to accurately position and resize the figure
        // to fit everything in the "size" given as input to this function.
        let yaxis = AxisTransform::id(vtype, vunit);
        let ticks = XYTicks::prepare(ui, xlims, ylims, axis, Some(&yaxis));
        let x_labels_height = ticks.x_labels_height();
        let y_labels_width = ticks.y_labels_width();

        const BOTTOM_PADDING: f32 = 40.0;
        const RIGHT_PADDING: f32 = 20.0;
        let size = [
            size[0] - y_labels_width - RIGHT_PADDING,
            size[1] - x_labels_height - BOTTOM_PADDING,
        ];

        // Start drawing the figure
        ui.set_cursor_screen_pos([pos[0] + y_labels_width, pos[1]]);
        let p = ui.cursor_screen_pos();

        ui.invisible_button(format!("plot"), size);
        let is_plot_hovered = ui.is_item_hovered();
        ui.set_item_allow_overlap();

        let draw_list = ui.get_window_draw_list();

        const BG_COLOR: u32 = 0xA033_3333;
        const LINE_COLOR: u32 = 0xFFFF_FFFF;

        let bottom_right_corner = [p[0] + size[0], p[1] + size[1]];

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
                    p[0] + (x1 - xlims.0) / (xlims.1 - xlims.0) * size[0],
                    p[1] + size[1] - (y1 - ylims.0) / (ylims.1 - ylims.0) * size[1],
                ];
                let p1 = [
                    p[0] + (x2 - xlims.0) / (xlims.1 - xlims.0) * size[0],
                    p[1] + size[1] - (y2 - ylims.0) / (ylims.1 - ylims.0) * size[1],
                ];
                draw_list.add_line(p0, p1, LINE_COLOR).build();
            }
        });

        let mouse_x = ui.io().mouse_pos[0];
        self.mouse_pos[0] = xlims.0 + (mouse_x - p[0]) / size[0] * (xlims.1 - xlims.0);
        if is_plot_hovered {
            let point = self.mouse_pos[0] as usize;
            if let Some(y) = image.get(point) {
                let x = axis.map(|axis| Measurement {
                    v: axis.pix2world(self.mouse_pos[0]),
                    unit: axis.unit(),
                });
                let val = Measurement { v: *y, unit: vunit };
                let text = self.make_tooltip(point, x, val);
                ui.tooltip_text(text);
            }

            // Zoom along X-axis
            let wheel_delta = ui.io().mouse_wheel;

            if wheel_delta != 0.0 {
                const ZOOM_SPEED: f32 = 0.5;

                let zoom_before = self.zoom[0];
                self.zoom[0] *= 1.0 - wheel_delta * ZOOM_SPEED;
                // Correct offset value so that the zoom be centered on the mouse position
                self.offset[0] -= (self.zoom[0] - zoom_before)
                    * (xvlims.0 + (mouse_x - p[0]) / size[0] * (xvlims.1 - xvlims.0));
            }

            // Pan by dragging mouse
            if !self.interactions.any_moving() && ui.is_mouse_dragging(MouseButton::Left) {
                ui.set_mouse_cursor(Some(MouseCursor::ResizeAll));
                let delta = ui.io().mouse_delta;
                self.offset[0] -= delta[0] / size[0] * (xlims.1 - xlims.0);
                self.offset[1] += delta[1] / size[1] * (ylims.1 - ylims.0);
            }

            if ui.is_mouse_clicked(MouseButton::Right) {
                ui.open_popup(format!("add-interaction-handle"))
            }
        }

        let mut line_marked_for_deletion = None;
        // Flag to only allow line to be moved at a time
        let mut moved_one = false;
        for (id, interaction) in self.interactions.iter_mut() {
            let stack = ui.push_id(id.id());
            match interaction {
                Interaction::VerticalLine(VerticalLine { x_pos, moving }) => {
                    const LINE_COLOR: u32 = 0xFFFF_FFFF;
                    const LINE_LABEL_LELT_PADDING: f32 = 10.0;
                    const LINE_LABEL_TOP_PADDING: f32 = 10.0;

                    let x = p[0] + (*x_pos - xlims.0) / (xlims.1 - xlims.0) * size[0];
                    let y = p[1];

                    const CLICKABLE_WIDTH: f32 = 5.0;

                    ui.set_cursor_screen_pos([x - CLICKABLE_WIDTH, y]);
                    ui.invisible_button(format!("vertical-line"), [2.0 * CLICKABLE_WIDTH, size[1]]);
                    if ui.is_item_hovered() {
                        ui.set_mouse_cursor(Some(MouseCursor::ResizeEW));
                        if ui.is_mouse_clicked(MouseButton::Left) {
                            *moving = true;
                        }
                        if ui.is_mouse_clicked(MouseButton::Right) {
                            ui.open_popup(format!("edit-vertical-line"))
                        }
                    }
                    if !moved_one && *moving {
                        moved_one = true;
                        *x_pos = util::clamp(self.mouse_pos[0].round(), xvlims.0, xvlims.1);
                    }
                    if !ui.is_mouse_down(MouseButton::Left) {
                        *moving = false;
                    }

                    draw_list
                        .add_line([x, y], [x, y + size[1]], LINE_COLOR)
                        .build();
                    draw_list.add_text(
                        [x + LINE_LABEL_LELT_PADDING, y + LINE_LABEL_TOP_PADDING],
                        LINE_COLOR,
                        &format!("{:.0}", x_pos),
                    );

                    ui.popup(format!("edit-vertical-line"), || {
                        if MenuItem::new(format!("Delete Line")).build(ui) {
                            line_marked_for_deletion = Some(*id);
                        }
                        if MenuItem::new(format!("Copy Line")).build(ui) {
                            if store.contains_key(id) {
                                let t_idx = *store.get(id).unwrap();
                                *copying = Some((*id, t_idx));
                            } else {
                                println!("copy failued");
                            }
                        }
                    });
                }
                // Unused in plot
                Interaction::HorizontalLine(_) => {}
                Interaction::FinedGrainedROI(_) => {}
                Interaction::Line(_) => {}
                Interaction::Circle(_) => {}
                Interaction::Lims(_) => {}
                Interaction::ColorLims(_) => {}
            }
            stack.pop();
        }

        if let Some(line_id) = line_marked_for_deletion {
            self.interactions.remove(line_id);
        }

        ticks.draw(&draw_list, p, size);

        // Add interaction handlers
        ui.popup(format!("add-interaction-handle"), || {
            ui.text("Add interaction handle");
            ui.separator();
            if let Some(menu) = ui.begin_menu_with_enabled(format!("Vertical Line"), true) {
                if MenuItem::new(format!("to main editor")).build(ui) {
                    let new =
                        Interaction::VerticalLine(VerticalLine::new(self.mouse_pos[0].round()));
                    self.interactions.insert(new);
                }
                for macr in node_editor.macros.macros() {
                    if MenuItem::new(&format!("to macro: {}", macr.name())).build(ui) {
                        let new =
                            Interaction::VerticalLine(VerticalLine::new(self.mouse_pos[0].round()));
                        self.interactions.insert(new);
                        let macro_id = macr.id();
                        let mut dstw = macr.write();
                        let t_idx = dstw.dst_mut().add_owned_transform(
                            Transform::new_constant(aflak_primitives::IOValue::Float(
                                self.mouse_pos[0].round(),
                            )),
                            Some(macro_id),
                        );
                        drop(dstw);
                        let t_idx = t_idx.set_macro(macro_id);
                        store.insert(self.interactions.id(), t_idx);
                    }
                }
                menu.end();
            }
            if let Some((_, t_idx)) = *copying {
                ui.separator();
                ui.text("Paste Line Options");
                ui.separator();
                if MenuItem::new(format!("Paste Line as Vertical Line")).build(ui) {
                    let new =
                        Interaction::VerticalLine(VerticalLine::new(self.mouse_pos[0].round()));
                    self.interactions.insert(new);
                    store.insert(self.interactions.id(), t_idx);
                    *copying = None;
                }
            }
            ui.separator();
            if MenuItem::new(format!("Reset view")).build(ui) {
                self.zoom = [1.0, 1.0];
                self.offset = [0.0, 0.0];
            }
        });
        if let Some((o, t_idx, kind)) = *attaching {
            if kind == 1 && o == outputid {
                let mut already_insert = false;
                for d in store.iter() {
                    if *d.1 == t_idx {
                        already_insert = true;
                        break;
                    }
                }
                if !already_insert {
                    let new =
                        Interaction::VerticalLine(VerticalLine::new(self.mouse_pos[0].round()));
                    self.interactions.insert(new);
                    store.insert(self.interactions.id(), t_idx);
                } else {
                    eprintln!("{:?} is already bound", t_idx)
                }
                *attaching = None;
            }
        }

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
