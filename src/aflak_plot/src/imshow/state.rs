use glium::backend::Facade;
use imgui::{ImGuiMouseCursor, ImMouseButton, ImStr, ImVec2, Ui};
use imgui_glium_renderer::Texture;
use ndarray::Array2;

use super::hist;
use super::image;
use super::interactions::{
    HorizontalLine, Interaction, InteractionIterMut, Interactions, ValueIter, VerticalLine,
};
use super::lut::{BuiltinLUT, ColorLUT};
use super::ticks;
use super::util;
use super::Error;

/// Current state of the visualization of a 2D image
pub struct State {
    pub(crate) lut: ColorLUT,
    pub(crate) vmin: f32,
    pub(crate) vmax: f32,
    /// Mouse position relative to the image (in pixels)
    pub mouse_pos: (f32, f32),
    /// Control whether histogram uses a log scale
    pub hist_logscale: bool,
    lut_min_moving: bool,
    lut_max_moving: bool,
    interactions: Interactions,
}

impl Default for State {
    fn default() -> Self {
        use std::f32;
        Self {
            lut: BuiltinLUT::Flame.lut(),
            vmin: f32::NAN,
            vmax: f32::NAN,
            mouse_pos: (f32::NAN, f32::NAN),
            hist_logscale: true,
            lut_min_moving: false,
            lut_max_moving: false,
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

    pub(crate) fn show_bar<P, S>(&mut self, ui: &Ui, pos: P, size: S)
    where
        P: Into<ImVec2>,
        S: Into<ImVec2>,
    {
        let pos = pos.into();
        let size = size.into();

        ui.set_cursor_screen_pos(pos);
        ui.invisible_button(im_str!("image_bar"), size);
        if ui.is_item_hovered() {
            if ui.imgui().is_mouse_clicked(ImMouseButton::Right) {
                ui.open_popup(im_str!("swap-lut"))
            }
        }
        ui.popup(im_str!("swap-lut"), || {
            ui.text("Swap LUT");
            ui.separator();
            for builtin_lut in BuiltinLUT::values() {
                ui.push_id(*builtin_lut as i32);
                if ui.menu_item(builtin_lut.name()).build() {
                    self.lut.set_gradient(*builtin_lut);
                }
                ui.pop_id();
            }
        });

        let draw_list = ui.get_window_draw_list();

        // Show triangle to change contrast
        {
            const TRIANGLE_LEFT_PADDING: f32 = 10.0;
            const TRIANGLE_HEIGHT: f32 = 20.0;
            const TRIANGLE_WIDTH: f32 = 15.0;
            let lims = self.lut.lims();

            // Min triangle
            let min_color = util::to_u32_color(&self.lut.color_at(lims.0));
            let x_pos = pos.x + size.x + TRIANGLE_LEFT_PADDING;
            let y_pos = pos.y + size.y * (1.0 - lims.0);
            draw_list
                .add_triangle(
                    [x_pos, y_pos],
                    [x_pos + TRIANGLE_WIDTH, y_pos + TRIANGLE_HEIGHT / 2.0],
                    [x_pos + TRIANGLE_WIDTH, y_pos - TRIANGLE_HEIGHT / 2.0],
                    min_color,
                ).filled(true)
                .build();
            draw_list
                .add_triangle(
                    [x_pos, y_pos],
                    [x_pos + TRIANGLE_WIDTH, y_pos + TRIANGLE_HEIGHT / 2.0],
                    [x_pos + TRIANGLE_WIDTH, y_pos - TRIANGLE_HEIGHT / 2.0],
                    util::invert_color(min_color),
                ).build();
            if lims.0 != 0.0 {
                let min_threshold = util::lerp(self.vmin, self.vmax, lims.0);
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
            }
            if !ui.imgui().is_mouse_down(ImMouseButton::Left) {
                self.lut_min_moving = false;
            }

            // Max triangle
            let max_color = util::to_u32_color(&self.lut.color_at(lims.1));
            let y_pos = pos.y + size.y * (1.0 - lims.1);
            draw_list
                .add_triangle(
                    [x_pos, y_pos],
                    [x_pos + TRIANGLE_WIDTH, y_pos + TRIANGLE_HEIGHT / 2.0],
                    [x_pos + TRIANGLE_WIDTH, y_pos - TRIANGLE_HEIGHT / 2.0],
                    max_color,
                ).filled(true)
                .build();
            draw_list
                .add_triangle(
                    [x_pos, y_pos],
                    [x_pos + TRIANGLE_WIDTH, y_pos + TRIANGLE_HEIGHT / 2.0],
                    [x_pos + TRIANGLE_WIDTH, y_pos - TRIANGLE_HEIGHT / 2.0],
                    util::invert_color(max_color),
                ).build();
            if lims.1 != 1.0 {
                let max_threshold = util::lerp(self.vmin, self.vmax, lims.1);
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
            }
            if !ui.imgui().is_mouse_down(ImMouseButton::Left) {
                self.lut_max_moving = false;
            }
        }

        let x_pos = pos.x + 5.0;
        for ((v1, c1), (v2, c2)) in self.lut.bounds() {
            let bottom_col = util::to_u32_color(&c1);
            let top_col = util::to_u32_color(&c2);
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
        const COLOR: u32 = 0xFFFFFFFF;
        const TICK_SIZE: f32 = 3.0;
        const TICK_COUNT: usize = 10;
        const TICK_STEP: f32 = 1.0 / TICK_COUNT as f32;
        while i >= -0.01 {
            let tick_y_pos = util::lerp(pos.y, pos.y + size.y, i);
            let y_pos = tick_y_pos - text_height / 2.5;
            let val = self.vmax + (self.vmin - self.vmax) * i;
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
                ).build();
            // TODO: Make step editable
            i -= TICK_STEP;
        }
    }

    pub(crate) fn show_image<F>(
        &mut self,
        ui: &Ui,
        ctx: &F,
        name: &ImStr,
        image: &Array2<f32>,
        max_size: (f32, f32),
    ) -> Result<[(f32, f32); 2], Error>
    where
        F: Facade,
    {
        const IMAGE_LEFT_PADDING: f32 = 20.0;
        const IMAGE_TOP_PADDING: f32 = 0.0;

        // Returns a handle to the mutable texture (we could write on it)
        let raw = image::make_raw_image(image, self)?;
        let texture = ui.replace_texture(name, Texture::from_data(ctx, raw)?);
        let tex_size = texture.get_size();
        let size = {
            const BOTTOM_PADDING: f32 = 60.0;
            const MIN_WIDTH: f32 = 100.0;
            const MIN_HEIGHT: f32 = 100.0;
            let available_size = (
                MIN_WIDTH.max(max_size.0 - IMAGE_LEFT_PADDING),
                MIN_HEIGHT.max(max_size.1 - BOTTOM_PADDING - IMAGE_TOP_PADDING),
            );
            let original_size = (tex_size.0 as f32, tex_size.1 as f32);
            let zoom = (available_size.0 / original_size.0).min(available_size.1 / original_size.1);
            (original_size.0 * zoom, original_size.1 * zoom)
        };

        let p = ui.get_cursor_screen_pos();
        ui.set_cursor_screen_pos([p.0 + IMAGE_LEFT_PADDING, p.1 + IMAGE_TOP_PADDING]);
        let p = ui.get_cursor_screen_pos();

        ui.image(&texture, size).build();
        let abs_mouse_pos = ui.imgui().mouse_pos();
        let mouse_pos = (abs_mouse_pos.0 - p.0, -abs_mouse_pos.1 + p.1 + size.1);
        self.mouse_pos = (
            mouse_pos.0 / size.0 * tex_size.0 as f32,
            mouse_pos.1 / size.1 * tex_size.1 as f32,
        );

        if ui.is_item_hovered() {
            let x = self.mouse_pos.0 as usize;
            let y = self.mouse_pos.1 as usize;
            if y < image.dim().1 {
                let index = [image.dim().1 - 1 - y, x];
                if let Some(val) = image.get(index) {
                    ui.tooltip_text(format!("X: {}, Y: {}, VAL: {:.2}", x, y, val));
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
                let new = Interaction::HorizontalLine(HorizontalLine::new(self.mouse_pos.1));
                self.interactions.insert(new);
            }
            if ui.menu_item(im_str!("Vertical Line")).build() {
                let new = Interaction::VerticalLine(VerticalLine::new(self.mouse_pos.0));
                self.interactions.insert(new);
            }
        });

        let mut line_marked_for_deletion = None;
        for (id, interaction) in self.interactions.iter_mut() {
            ui.push_id(id.id());
            const LINE_COLOR: u32 = 0xFFFFFFFF;
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
                        *height = util::clamp(self.mouse_pos.1, 0.0, tex_size.1 as f32);
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

                    ui.set_cursor_screen_pos([x, y - CLICKABLE_WIDTH]);

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
                        *x_pos = util::clamp(self.mouse_pos.0, 0.0, tex_size.0 as f32);
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
            }
            ui.pop_id();
        }

        if let Some(line_id) = line_marked_for_deletion {
            self.interactions.remove(&line_id);
        }

        ticks::add_ticks(
            ui,
            &draw_list,
            p,
            size,
            (0.0, tex_size.0 as f32),
            (0.0, tex_size.1 as f32),
        );

        Ok([p, size])
    }

    pub(crate) fn show_hist<P, S>(&self, ui: &Ui, pos: P, size: S, image: &Array2<f32>)
    where
        P: Into<ImVec2>,
        S: Into<ImVec2>,
    {
        let pos = pos.into();
        let size = size.into();

        const FILL_COLOR: u32 = 0xFF999999;
        const BORDER_COLOR: u32 = 0xFF000000;
        let hist = hist::histogram(image, self.vmin, self.vmax);
        if let Some(max_count) = hist.iter().map(|bin| bin.count).max() {
            let draw_list = ui.get_window_draw_list();

            let x_pos = pos.x;
            for bin in hist {
                let y_pos = pos.y + size.y / (self.vmax - self.vmin) * (self.vmax - bin.start);
                let y_pos_end = pos.y + size.y / (self.vmax - self.vmin) * (self.vmax - bin.end);
                let length = size.x * if self.hist_logscale {
                    (bin.count as f32).log10() / (max_count as f32).log10()
                } else {
                    (bin.count as f32) / (max_count as f32)
                };
                draw_list
                    .add_rect(
                        [x_pos + size.x - length, y_pos],
                        [x_pos + size.x, y_pos_end],
                        FILL_COLOR,
                    ).filled(true)
                    .build();
            }

            draw_list
                .add_rect(pos, [pos.x + size.x, pos.y + size.y], BORDER_COLOR)
                .build();
        } // TODO show error
    }
}
