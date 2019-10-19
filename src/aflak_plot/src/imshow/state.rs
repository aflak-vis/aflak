use std::borrow::Borrow;
use std::time::Instant;

use glium::backend::Facade;
use imgui::{ImGuiMouseCursor, ImMouseButton, ImString, ImTexture, ImVec2, Ui};
use ndarray::ArrayD;

use super::image;
use super::interactions::{
    FinedGrainedROI, HorizontalLine, Interaction, InteractionIterMut, Interactions, Line,
    ValueIter, VerticalLine,
};
use super::lut::{BuiltinLUT, ColorLUT};
use super::ticks::XYTicks;
use super::util;
use super::AxisTransform;
use super::Error;
use super::Textures;

/// Current state of the visualization of a 2D image
pub struct State<I> {
    pub(crate) lut: ColorLUT,
    /// Mouse position relative to the image (in pixels)
    pub mouse_pos: (f32, f32),
    /// Control whether histogram uses a log scale
    pub hist_logscale: bool,
    lut_min_moving: bool,
    lut_max_moving: bool,
    interactions: Interactions,
    roi_input: RoiInputState,
    line_input: LineInputState,
    image: image::Image<I>,
}

#[derive(Default)]
struct RoiInputState {
    roi_id_cnt: usize,
    roi_names: Vec<ImString>,
    selected: i32,
}

impl RoiInputState {
    fn gen_id(&mut self) -> usize {
        self.roi_id_cnt += 1;
        self.roi_names
            .push(ImString::new(format!("Roi {}", self.roi_id_cnt)));
        self.selected = self.roi_id_cnt as i32;
        self.roi_id_cnt
    }

    fn is_selected(&self, id: usize) -> bool {
        id as i32 == self.selected
    }
}

#[derive(Default)]
struct LineInputState {
    line_id_cnt: usize,
    line_names: Vec<ImString>,
    selected: i32,
}

impl LineInputState {
    fn gen_id(&mut self) -> usize {
        self.line_id_cnt += 1;
        self.line_names
            .push(ImString::new(format!("Line: {}", self.line_id_cnt)));
        self.selected = self.line_id_cnt as i32;
        self.line_id_cnt
    }

    fn is_selected(&self, id: usize) -> bool {
        id as i32 == self.selected
    }
}

impl<I> Default for State<I> {
    fn default() -> Self {
        use std::f32;
        Self {
            lut: BuiltinLUT::Flame.lut(),
            mouse_pos: (f32::NAN, f32::NAN),
            hist_logscale: true,
            lut_min_moving: false,
            lut_max_moving: false,
            interactions: Interactions::new(),
            roi_input: Default::default(),
            line_input: Default::default(),
            image: Default::default(),
        }
    }
}

impl<I> State<I>
where
    I: Borrow<ArrayD<f32>>,
{
    pub fn stored_values(&self) -> ValueIter {
        self.interactions.value_iter()
    }

    pub fn stored_values_mut(&mut self) -> InteractionIterMut {
        self.interactions.iter_mut()
    }

    pub fn set_image<F>(
        &mut self,
        image: I,
        created_on: Instant,
        ctx: &F,
        texture_id: ImTexture,
        textures: &mut Textures,
    ) -> Result<(), Error>
    where
        F: Facade,
    {
        self.image = image::Image::new(image, created_on, ctx, texture_id, textures, &self.lut)?;
        Ok(())
    }

    pub fn image_created_on(&self) -> Option<Instant> {
        self.image.created_on()
    }

    pub(crate) fn image(&self) -> &image::Image<I> {
        &self.image
    }

    pub(crate) fn show_bar<P, S>(&mut self, ui: &Ui, pos: P, size: S) -> bool
    where
        P: Into<ImVec2>,
        S: Into<ImVec2>,
    {
        let mut changed = false;
        let pos = pos.into();
        let size = size.into();

        ui.set_cursor_screen_pos(pos);
        ui.invisible_button(im_str!("image_bar"), size);
        if ui.is_item_hovered() && ui.imgui().is_mouse_clicked(ImMouseButton::Right) {
            ui.open_popup(im_str!("swap-lut"));
        }
        ui.popup(im_str!("swap-lut"), || {
            ui.text("Swap LUT");
            ui.separator();
            for builtin_lut in BuiltinLUT::values() {
                ui.push_id(*builtin_lut as i32);
                if ui.menu_item(builtin_lut.name()).build() {
                    self.lut.set_gradient(*builtin_lut);
                    changed = true;
                }
                ui.pop_id();
            }
        });

        let draw_list = ui.get_window_draw_list();

        let vmin = self.image.vmin();
        let vmax = self.image.vmax();
        // Show triangle to change contrast
        {
            const TRIANGLE_LEFT_PADDING: f32 = 10.0;
            const TRIANGLE_HEIGHT: f32 = 20.0;
            const TRIANGLE_WIDTH: f32 = 15.0;
            let lims = self.lut.lims();

            // Min triangle
            let min_color = util::to_u32_color(self.lut.color_at(lims.0));
            let x_pos = pos.x + size.x + TRIANGLE_LEFT_PADDING;
            let y_pos = pos.y + size.y * (1.0 - lims.0);
            draw_list
                .add_triangle(
                    [x_pos, y_pos],
                    [x_pos + TRIANGLE_WIDTH, y_pos + TRIANGLE_HEIGHT / 2.0],
                    [x_pos + TRIANGLE_WIDTH, y_pos - TRIANGLE_HEIGHT / 2.0],
                    min_color,
                )
                .filled(true)
                .build();
            draw_list
                .add_triangle(
                    [x_pos, y_pos],
                    [x_pos + TRIANGLE_WIDTH, y_pos + TRIANGLE_HEIGHT / 2.0],
                    [x_pos + TRIANGLE_WIDTH, y_pos - TRIANGLE_HEIGHT / 2.0],
                    util::invert_color(min_color),
                )
                .build();
            if lims.0 != 0.0 {
                let min_threshold = util::lerp(vmin, vmax, lims.0);
                draw_list.add_text(
                    [x_pos + TRIANGLE_WIDTH + LABEL_HORIZONTAL_PADDING, y_pos],
                    COLOR,
                    &format!("{:.2}", min_threshold),
                );
            }
            ui.set_cursor_screen_pos([x_pos, y_pos - TRIANGLE_HEIGHT / 2.0]);
            ui.invisible_button(im_str!("set_min"), [TRIANGLE_WIDTH, TRIANGLE_HEIGHT]);
            if ui.is_item_hovered() {
                ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeNS);
                if ui.imgui().is_mouse_clicked(ImMouseButton::Left) {
                    self.lut_min_moving = true;
                }
            }
            if self.lut_min_moving {
                ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeNS);
                let (_, mouse_y) = ui.imgui().mouse_pos();
                let min = 1.0 - (mouse_y - pos.y) / size.y;
                self.lut.set_min(min);
                changed = true;
            }
            if !ui.imgui().is_mouse_down(ImMouseButton::Left) {
                self.lut_min_moving = false;
            }

            // Max triangle
            let max_color = util::to_u32_color(self.lut.color_at(lims.1));
            let y_pos = pos.y + size.y * (1.0 - lims.1);
            draw_list
                .add_triangle(
                    [x_pos, y_pos],
                    [x_pos + TRIANGLE_WIDTH, y_pos + TRIANGLE_HEIGHT / 2.0],
                    [x_pos + TRIANGLE_WIDTH, y_pos - TRIANGLE_HEIGHT / 2.0],
                    max_color,
                )
                .filled(true)
                .build();
            draw_list
                .add_triangle(
                    [x_pos, y_pos],
                    [x_pos + TRIANGLE_WIDTH, y_pos + TRIANGLE_HEIGHT / 2.0],
                    [x_pos + TRIANGLE_WIDTH, y_pos - TRIANGLE_HEIGHT / 2.0],
                    util::invert_color(max_color),
                )
                .build();
            if lims.1 < 1.0 {
                let max_threshold = util::lerp(vmin, vmax, lims.1);
                draw_list.add_text(
                    [x_pos + TRIANGLE_WIDTH + LABEL_HORIZONTAL_PADDING, y_pos],
                    COLOR,
                    &format!("{:.2}", max_threshold),
                );
            }
            ui.set_cursor_screen_pos([x_pos, y_pos - TRIANGLE_HEIGHT / 2.0]);
            ui.invisible_button(im_str!("set_max"), [TRIANGLE_WIDTH, TRIANGLE_HEIGHT]);
            if ui.is_item_hovered() {
                ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeNS);
                if ui.imgui().is_mouse_clicked(ImMouseButton::Left) {
                    self.lut_max_moving = true;
                }
            }
            if self.lut_max_moving {
                ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeNS);
                let (_, mouse_y) = ui.imgui().mouse_pos();
                let max = 1.0 - (mouse_y - pos.y) / size.y;
                self.lut.set_max(max);
                changed = true;
            }
            if !ui.imgui().is_mouse_down(ImMouseButton::Left) {
                self.lut_max_moving = false;
            }
        }

        let x_pos = pos.x + 5.0;
        for ((v1, c1), (v2, c2)) in self.lut.bounds() {
            let bottom_col = util::to_u32_color(c1);
            let top_col = util::to_u32_color(c2);
            let bottom_y_pos = pos.y + size.y * (1.0 - v1);
            let top_y_pos = pos.y + size.y * (1.0 - v2);
            draw_list.add_rect_filled_multicolor(
                [x_pos, top_y_pos],
                [x_pos + size.x, bottom_y_pos],
                top_col,
                top_col,
                bottom_col,
                bottom_col,
            );
        }
        let mut i = 1.0;
        let text_height = ui.get_text_line_height_with_spacing();
        const LABEL_HORIZONTAL_PADDING: f32 = 2.0;
        const COLOR: u32 = 0xFFFF_FFFF;
        const TICK_SIZE: f32 = 3.0;
        const TICK_COUNT: usize = 10;
        const TICK_STEP: f32 = 1.0 / TICK_COUNT as f32;
        while i >= -0.01 {
            let tick_y_pos = util::lerp(pos.y, pos.y + size.y, i);
            let y_pos = tick_y_pos - text_height / 2.5;
            let val = vmax + (vmin - vmax) * i;
            draw_list.add_text(
                [x_pos + size.x + LABEL_HORIZONTAL_PADDING, y_pos],
                COLOR,
                &format!("{:.2}", val),
            );
            draw_list
                .add_line(
                    [x_pos + size.x - TICK_SIZE, tick_y_pos],
                    [x_pos + size.x, tick_y_pos],
                    COLOR,
                )
                .build();
            // TODO: Make step editable
            i -= TICK_STEP;
        }

        changed
    }

    pub(crate) fn show_image<FX, FY>(
        &mut self,
        ui: &Ui,
        texture_id: ImTexture,
        vunit: &str,
        xaxis: Option<&AxisTransform<FX>>,
        yaxis: Option<&AxisTransform<FY>>,
        max_size: (f32, f32),
    ) -> Result<([(f32, f32); 2], f32), Error>
    where
        FX: Fn(f32) -> f32,
        FY: Fn(f32) -> f32,
    {
        const IMAGE_TOP_PADDING: f32 = 0.0;

        let tex_size = self.image.tex_size();
        let ticks = XYTicks::prepare(
            ui,
            (0.0, tex_size.0 as f32),
            (0.0, tex_size.1 as f32),
            xaxis,
            yaxis,
        );
        let x_labels_height = ticks.x_labels_height();
        let y_labels_width = ticks.y_labels_width();

        let size = {
            const MIN_WIDTH: f32 = 100.0;
            const MIN_HEIGHT: f32 = 100.0;
            let available_size = (
                MIN_WIDTH.max(max_size.0 - y_labels_width),
                MIN_HEIGHT.max(max_size.1 - x_labels_height - IMAGE_TOP_PADDING),
            );
            let original_size = (tex_size.0 as f32, tex_size.1 as f32);
            let zoom = (available_size.0 / original_size.0).min(available_size.1 / original_size.1);
            (original_size.0 * zoom, original_size.1 * zoom)
        };

        let p = ui.get_cursor_screen_pos();
        ui.set_cursor_screen_pos([p.0 + y_labels_width, p.1 + IMAGE_TOP_PADDING]);
        let p = ui.get_cursor_screen_pos();

        ui.image(texture_id, size).build();
        let is_image_hovered = ui.is_item_hovered();

        let abs_mouse_pos = ui.imgui().mouse_pos();
        let mouse_pos = (abs_mouse_pos.0 - p.0, -abs_mouse_pos.1 + p.1 + size.1);
        self.mouse_pos = (
            mouse_pos.0 / size.0 * tex_size.0 as f32,
            mouse_pos.1 / size.1 * tex_size.1 as f32,
        );

        if is_image_hovered {
            let x = self.mouse_pos.0 as usize;
            let y = self.mouse_pos.1 as usize;
            if y < self.image.dim().0 {
                let index = [self.image.dim().0 - 1 - y, x];
                if let Some(val) = self.image.get(index) {
                    let x_measurement = xaxis.map(|axis| Measurement {
                        v: axis.pix2world(x as f32),
                        unit: axis.unit(),
                    });
                    let y_measurement = yaxis.map(|axis| Measurement {
                        v: axis.pix2world(y as f32),
                        unit: axis.unit(),
                    });
                    let text = self.make_tooltip(
                        (x, y),
                        x_measurement,
                        y_measurement,
                        Measurement {
                            v: val,
                            unit: vunit,
                        },
                    );
                    ui.tooltip_text(text);
                }
            }

            if ui.imgui().is_mouse_clicked(ImMouseButton::Right) {
                ui.open_popup(im_str!("add-interaction-handle"))
            }
        }

        let draw_list = ui.get_window_draw_list();

        // Add interaction handlers
        ui.popup(im_str!("add-interaction-handle"), || {
            ui.text("Add interaction handle");
            ui.separator();
            if ui.menu_item(im_str!("Horizontal Line")).build() {
                let new =
                    Interaction::HorizontalLine(HorizontalLine::new(self.mouse_pos.1.round()));
                self.interactions.insert(new);
            }
            if ui.menu_item(im_str!("Vertical Line")).build() {
                let new = Interaction::VerticalLine(VerticalLine::new(self.mouse_pos.0.round()));
                self.interactions.insert(new);
            }
            if ui.menu_item(im_str!("Region of interest")).build() {
                let new =
                    Interaction::FinedGrainedROI(FinedGrainedROI::new(self.roi_input.gen_id()));
                self.interactions.insert(new);
            }
            if ui.menu_item(im_str!("Line")).build() {
                let new = Interaction::Line(Line::new(self.line_input.gen_id()));
                self.interactions.insert(new);
            }
        });

        let mut line_marked_for_deletion = None;
        for (id, interaction) in self.interactions.iter_mut() {
            ui.push_id(id.id());
            const LINE_COLOR: u32 = 0xFFFF_FFFF;
            match interaction {
                Interaction::HorizontalLine(HorizontalLine { height, moving }) => {
                    let x = p.0;
                    let y = p.1 + size.1 - *height / tex_size.1 as f32 * size.1;

                    const CLICKABLE_HEIGHT: f32 = 5.0;

                    ui.set_cursor_screen_pos([x, y - CLICKABLE_HEIGHT]);

                    ui.invisible_button(
                        im_str!("horizontal-line"),
                        [size.0, 2.0 * CLICKABLE_HEIGHT],
                    );
                    if ui.is_item_hovered() {
                        ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeNS);
                        if ui.imgui().is_mouse_clicked(ImMouseButton::Left) {
                            *moving = true;
                        }
                        if ui.imgui().is_mouse_clicked(ImMouseButton::Right) {
                            ui.open_popup(im_str!("edit-horizontal-line"))
                        }
                    }
                    if *moving {
                        *height = util::clamp(self.mouse_pos.1.round(), 0.0, tex_size.1 as f32);
                    }
                    if !ui.imgui().is_mouse_down(ImMouseButton::Left) {
                        *moving = false;
                    }

                    draw_list
                        .add_line([x, y], [x + size.0, y], LINE_COLOR)
                        .build();

                    ui.popup(im_str!("edit-horizontal-line"), || {
                        if ui.menu_item(im_str!("Delete Line")).build() {
                            line_marked_for_deletion = Some(*id);
                        }
                    });
                }
                Interaction::VerticalLine(VerticalLine { x_pos, moving }) => {
                    let x = p.0 + *x_pos / tex_size.0 as f32 * size.0;
                    let y = p.1;

                    const CLICKABLE_WIDTH: f32 = 5.0;

                    ui.set_cursor_screen_pos([x - CLICKABLE_WIDTH, y]);

                    ui.invisible_button(im_str!("vertical-line"), [2.0 * CLICKABLE_WIDTH, size.1]);
                    if ui.is_item_hovered() {
                        ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeEW);
                        if ui.imgui().is_mouse_clicked(ImMouseButton::Left) {
                            *moving = true;
                        }
                        if ui.imgui().is_mouse_clicked(ImMouseButton::Right) {
                            ui.open_popup(im_str!("edit-vertical-line"))
                        }
                    }
                    if *moving {
                        *x_pos = util::clamp(self.mouse_pos.0.round(), 0.0, tex_size.0 as f32);
                    }
                    if !ui.imgui().is_mouse_down(ImMouseButton::Left) {
                        *moving = false;
                    }

                    draw_list
                        .add_line([x, y], [x, y + size.1], LINE_COLOR)
                        .build();

                    ui.popup(im_str!("edit-vertical-line"), || {
                        if ui.menu_item(im_str!("Delete Line")).build() {
                            line_marked_for_deletion = Some(*id);
                        }
                    });
                }
                Interaction::FinedGrainedROI(FinedGrainedROI { id, pixels }) => {
                    let selected = self.roi_input.is_selected(*id);

                    let pixel_size_x = size.0 / tex_size.0 as f32;
                    let pixel_size_y = size.1 / tex_size.1 as f32;
                    const ROI_COLOR_SELECTED: u32 = 0xA000_0000;
                    const ROI_COLOR_UNSELECTED: u32 = 0x5000_0000;

                    let roi_color = if selected {
                        ROI_COLOR_SELECTED
                    } else {
                        ROI_COLOR_UNSELECTED
                    };

                    for &(i, j) in pixels.iter() {
                        let i = i as f32;
                        let j = j as f32;
                        let x = p.0 + i / tex_size.0 as f32 * size.0;
                        let y = p.1 + size.1 - j / tex_size.1 as f32 * size.1;
                        draw_list.add_rect_filled_multicolor(
                            [x, y],
                            [x + pixel_size_x, y - pixel_size_y],
                            roi_color,
                            roi_color,
                            roi_color,
                            roi_color,
                        )
                    }

                    if selected
                        && is_image_hovered
                        && ui.imgui().is_mouse_clicked(ImMouseButton::Left)
                    {
                        let pixel = (self.mouse_pos.0 as usize, self.mouse_pos.1 as usize);
                        let some_position = pixels.iter().position(|&pixel_| pixel_ == pixel);
                        if let Some(position) = some_position {
                            pixels.remove(position);
                        } else {
                            pixels.push(pixel);
                        }
                    }
                }
                Interaction::Line(Line {
                    id,
                    endpoints,
                    endpointsfill,
                }) => {
                    let selected = self.line_input.is_selected(*id);

                    if selected
                        && is_image_hovered
                        && ui.imgui().is_mouse_clicked(ImMouseButton::Left)
                    {
                        let pixel = (self.mouse_pos.0 as usize, self.mouse_pos.1 as usize);
                        if endpointsfill.0 == false {
                            endpointsfill.0 = true;
                            endpoints.0 = pixel;
                        } else if endpointsfill.1 == false {
                            endpointsfill.1 = true;
                            endpoints.1 = pixel;
                        }
                    }
                    let x0 = p.0 + (endpoints.0).0 as f32 / tex_size.0 as f32 * size.0;
                    let y0 = p.1 + size.1 - (endpoints.0).1 as f32 / tex_size.1 as f32 * size.1;
                    let x1 = p.0 + (endpoints.1).0 as f32 / tex_size.0 as f32 * size.0;
                    let y1 = p.1 + size.1 - (endpoints.1).1 as f32 / tex_size.1 as f32 * size.1;
                    if endpointsfill.0 && endpointsfill.1 {
                        draw_list.add_line([x0, y0], [x1, y1], LINE_COLOR).build();
                    }
                }
            }
            ui.pop_id();
        }

        if let Some(line_id) = line_marked_for_deletion {
            self.interactions.remove(line_id);
        }

        ticks.draw(&draw_list, p, size);

        Ok(([p, size], x_labels_height))
    }

    pub(crate) fn show_hist<P, S>(&self, ui: &Ui, pos: P, size: S)
    where
        P: Into<ImVec2>,
        S: Into<ImVec2>,
    {
        let pos = pos.into();
        let size = size.into();

        let vmin = self.image.vmin();
        let vmax = self.image.vmax();

        const FILL_COLOR: u32 = 0xFF99_9999;
        const BORDER_COLOR: u32 = 0xFF00_0000;
        let hist = self.image.hist();
        if let Some(max_count) = hist.iter().map(|bin| bin.count).max() {
            let draw_list = ui.get_window_draw_list();

            let x_pos = pos.x;
            for bin in hist {
                let y_pos = pos.y + size.y / (vmax - vmin) * (vmax - bin.start);
                let y_pos_end = pos.y + size.y / (vmax - vmin) * (vmax - bin.end);
                let length = size.x
                    * if self.hist_logscale {
                        (bin.count as f32).log10() / (max_count as f32).log10()
                    } else {
                        (bin.count as f32) / (max_count as f32)
                    };
                draw_list
                    .add_rect(
                        [x_pos + size.x - length, y_pos],
                        [x_pos + size.x, y_pos_end],
                        FILL_COLOR,
                    )
                    .filled(true)
                    .build();
            }

            draw_list
                .add_rect(pos, [pos.x + size.x, pos.y + size.y], BORDER_COLOR)
                .build();
        } // TODO show error
    }

    pub(crate) fn show_roi_selector(&mut self, ui: &Ui) {
        let any_roi = self.interactions.iter_mut().filter_roi().any(|_| true);
        if any_roi {
            let mut names = vec![im_str!("None")];
            for name in self.roi_input.roi_names.iter() {
                names.push(&name);
            }
            ui.combo(
                im_str!("Active ROI"),
                &mut self.roi_input.selected,
                &names,
                -1,
            );
        }
    }

    fn make_tooltip(
        &self,
        (x_p, y_p): (usize, usize),
        x: Option<Measurement>,
        y: Option<Measurement>,
        val: Measurement,
    ) -> String {
        let xy_str = format!(
            "(X, Y): ({}, {})",
            if let Some(x) = x {
                if x.unit.is_empty() {
                    format!("{:.2}", x.v)
                } else {
                    format!("{:.2} {}", x.v, x.unit)
                }
            } else {
                format!("{}", x_p)
            },
            if let Some(y) = y {
                if y.unit.is_empty() {
                    format!("{:.2}", y.v)
                } else {
                    format!("{:.2} {}", y.v, y.unit)
                }
            } else {
                format!("{}", y_p)
            },
        );

        let val_str = if val.unit.is_empty() {
            format!("VAL:    {:.2}", val.v)
        } else {
            format!("VAL:    {:.2} {}", val.v, val.unit)
        };

        if x.is_some() || y.is_some() {
            format!("{} [at point ({}, {})]\n{}", xy_str, x_p, y_p, val_str)
        } else {
            format!("{}\n{}", xy_str, val_str)
        }
    }
}

#[derive(Copy, Clone)]
pub struct Measurement<'a> {
    pub v: f32,
    pub unit: &'a str,
}
