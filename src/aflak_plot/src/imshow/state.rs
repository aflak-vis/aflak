use super::cake::{OutputId, Transform, TransformIdx};
use super::node_editor::NodeEditor;
use super::primitives::{IOErr, IOValue};
use glium::backend::Facade;
use imgui::{
    ChildWindow, Condition, ImString, Image, MenuItem, MouseButton, MouseCursor, Slider, TextureId,
    Ui, Window,
};
use ndarray::ArrayD;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::time::Instant;

use super::image;
use super::interactions::{
    Circle, ColorLims, FinedGrainedROI, HorizontalLine, Interaction, InteractionId,
    InteractionIterMut, Interactions, Lims, Line, ValueIter, VerticalLine,
};
use super::lut::{BuiltinLUT, ColorLUT};
use super::ticks::XYTicks;
use super::util;
use super::AxisTransform;
use super::Error;
use super::Textures;

type EditableValues = HashMap<InteractionId, TransformIdx>;
type AflakNodeEditor = NodeEditor<IOValue, IOErr>;

/// Current state of the visualization of a 2D image
pub struct State<I> {
    pub(crate) lut: ColorLUT,
    pub(crate) lut_color: [ColorLUT; 3],
    /// Mouse position relative to the image (in pixels)
    pub mouse_pos: (f32, f32),
    /// Control whether histogram uses a log scale
    pub hist_logscale: bool,
    lut_min_moving: bool,
    lut_mid_moving: bool,
    lut_max_moving: bool,
    lut_min_moving_rgb: [bool; 3],
    lut_mid_moving_rgb: [bool; 3],
    lut_max_moving_rgb: [bool; 3],
    interactions: Interactions,
    roi_input: RoiInputState,
    circle_input: CircleInputState,
    image: image::Image<I>,
    pub show_approx_line: bool,
    pub show_axis_option: bool,
    pub use_ms_for_degrees: (bool, bool),
    pub relative_to_center_for_degrees: (bool, bool),
    offset: [f32; 2],
    pub zoomkind: [bool; 4],
    scrolling: [f32; 2],
    parent_offset: [f32; 2],
}

#[derive(Default)]
struct RoiInputState {
    roi_id_cnt: usize,
    roi_names: Vec<ImString>,
    selected: usize,
}

impl RoiInputState {
    fn gen_id(&mut self) -> usize {
        self.roi_id_cnt += 1;
        self.roi_names
            .push(ImString::new(format!("Roi {}", self.roi_id_cnt)));
        self.selected = self.roi_id_cnt;
        self.roi_id_cnt
    }

    fn is_selected(&self, id: usize) -> bool {
        id == self.selected
    }
}

#[derive(Default)]
struct CircleInputState {
    circle_id_cnt: usize,
    circle_names: Vec<ImString>,
    selected: i32,
}

impl CircleInputState {
    fn gen_id(&mut self) -> usize {
        self.circle_id_cnt += 1;
        self.circle_names
            .push(ImString::new(format!("Circle: {}", self.circle_id_cnt)));
        self.selected = self.circle_id_cnt as i32;
        self.circle_id_cnt
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
            lut_color: [
                BuiltinLUT::Red.lut(),
                BuiltinLUT::Green.lut(),
                BuiltinLUT::Blue.lut(),
            ],
            mouse_pos: (f32::NAN, f32::NAN),
            hist_logscale: true,
            lut_min_moving: false,
            lut_mid_moving: false,
            lut_max_moving: false,
            lut_min_moving_rgb: [false; 3],
            lut_mid_moving_rgb: [false; 3],
            lut_max_moving_rgb: [false; 3],
            interactions: Interactions::new(),
            roi_input: Default::default(),
            circle_input: Default::default(),
            image: Default::default(),
            show_approx_line: false,
            show_axis_option: false,
            use_ms_for_degrees: (true, true),
            relative_to_center_for_degrees: (false, false),
            offset: [0.0, 0.0],
            zoomkind: [true, false, false, false],
            scrolling: [0.0, 0.0],
            parent_offset: [0.0, 0.0],
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

    pub fn zoom_init(&mut self) {
        self.offset = [0.0, 0.0];
        self.zoomkind = [true, false, false, false];
        self.scrolling = [0.0, 0.0];
        self.parent_offset = [0.0, 0.0];
    }

    pub fn set_image<F>(
        &mut self,
        image: I,
        created_on: Instant,
        ctx: &F,
        texture_id: TextureId,
        textures: &mut Textures,
    ) -> Result<(), Error>
    where
        F: Facade,
    {
        self.image = image::Image::new(image, created_on, ctx, texture_id, textures, &self.lut)?;
        Ok(())
    }

    pub fn set_color_image<F>(
        &mut self,
        image: I,
        created_on: Instant,
        ctx: &F,
        texture_id: TextureId,
        textures: &mut Textures,
    ) -> Result<(), Error>
    where
        F: Facade,
    {
        self.image = image::Image::color_new(
            image,
            created_on,
            ctx,
            texture_id,
            textures,
            &self.lut_color,
        )?;
        Ok(())
    }

    pub fn image_created_on(&self) -> Option<Instant> {
        self.image.created_on()
    }

    pub(crate) fn image(&self) -> &image::Image<I> {
        &self.image
    }

    pub(crate) fn show_bar(&mut self, ui: &Ui, pos: [f32; 2], size: [f32; 2]) -> bool {
        let mut changed = false;

        ui.set_cursor_screen_pos([pos[0] + 5.0, pos[1]]);
        ui.invisible_button(format!("image_bar"), size);
        if ui.is_item_hovered() && ui.is_mouse_clicked(MouseButton::Right) {
            ui.open_popup(format!("swap-lut"));
        }
        ui.popup(format!("swap-lut"), || {
            ui.text("Swap LUT");
            ui.separator();
            for builtin_lut in BuiltinLUT::values() {
                let stack = ui.push_id(*builtin_lut as i32);
                if MenuItem::new(builtin_lut.name()).build(ui) {
                    let (buf_min, buf_mid, buf_max) = self.lut.lims();
                    self.lut.set_lims(0.0, 0.5, 1.0);
                    self.lut.set_gradient(*builtin_lut);
                    self.lut.set_lims(buf_min, buf_mid, buf_max);
                    changed = true;
                }
                stack.pop();
            }
            ui.separator();
            ui.text("Stretch");
            ui.separator();
            if MenuItem::new("ZScale").build(ui) {
                let zscales = self.image.zscale(1000, 0.25);
                if let (Some(z1), Some(z2)) = zscales {
                    let vmin = self.image.vmin();
                    let vmax = self.image.vmax();
                    let lut_min = (z1 - vmin) / (vmax - vmin);
                    let lut_max = (z2 - vmin) / (vmax - vmin);
                    self.lut.set_lims(lut_min, 0.5, lut_max);
                    changed = true;
                }
            }
            if MenuItem::new("Auto").build(ui) {
                let med = self.image.vmed();
                let mad = self.image.vmad();
                let lut_min = if 1.0 + mad != 1.0 {
                    let b = med + -2.8 * mad;
                    util::clamp(b, 0.0, 1.0)
                } else {
                    0.0
                };
                let lut_mid = self.lut.mtf(0.25, med - lut_min);
                self.lut.set_lims(lut_min, lut_mid, 1.0);
                changed = true;
            }
            if MenuItem::new("Reset").build(ui) {
                self.lut.set_lims(0.0, 0.5, 1.0);
                changed = true;
            }
            if let Some(menu) = ui.begin_menu_with_enabled(format!("Lims"), true) {
                if MenuItem::new(format!("to main editor")).build(ui) {
                    let now_lims = self.lut.lims();
                    let now_lims = [now_lims.0, now_lims.1, now_lims.2];
                    let new = Interaction::Lims(Lims::new(now_lims));
                    self.interactions.insert(new);
                }
                menu.end();
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
            let x_pos = pos[0] + size[0] + TRIANGLE_LEFT_PADDING;
            let y_pos = pos[1] + size[1] * (1.0 - lims.0);
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
            ui.invisible_button(format!("set_min"), [TRIANGLE_WIDTH, TRIANGLE_HEIGHT]);
            let mut hovered_or_moving = false;
            if ui.is_item_hovered() {
                hovered_or_moving = true;
                ui.set_mouse_cursor(Some(MouseCursor::ResizeNS));
                if ui.is_mouse_clicked(MouseButton::Left) {
                    self.lut_min_moving = true;
                }
            }
            if self.lut_min_moving {
                hovered_or_moving = true;
                ui.set_mouse_cursor(Some(MouseCursor::ResizeNS));
                let [_, mouse_y] = ui.io().mouse_pos;
                let min = 1.0 - (mouse_y - pos[1]) / size[1];
                self.lut.set_min(min);
                changed = true;
            }
            if hovered_or_moving {
                let p = self.lut.lims().0;
                let val = vmin + (vmax - vmin) * p;
                ui.tooltip_text(format!("shadow"));
                ui.tooltip_text(format!("LIM: {:.6}", p));
                ui.tooltip_text(format!("VAL: {:.6}", val));
            }
            if !ui.is_mouse_down(MouseButton::Left) {
                self.lut_min_moving = false;
            }

            // Mid_tone triangle
            let mid_color =
                util::to_u32_color(self.lut.color_at(lims.0 + (lims.2 - lims.0) * lims.1));
            let x_pos = pos[0] + size[0] + TRIANGLE_LEFT_PADDING;
            let y_pos = pos[1] + size[1] * (1.0 - (lims.0 + (lims.2 - lims.0) * lims.1));
            draw_list
                .add_triangle(
                    [x_pos, y_pos],
                    [x_pos + TRIANGLE_WIDTH, y_pos + TRIANGLE_HEIGHT / 2.0],
                    [x_pos + TRIANGLE_WIDTH, y_pos - TRIANGLE_HEIGHT / 2.0],
                    mid_color,
                )
                .filled(true)
                .build();
            draw_list
                .add_triangle(
                    [x_pos, y_pos],
                    [x_pos + TRIANGLE_WIDTH, y_pos + TRIANGLE_HEIGHT / 2.0],
                    [x_pos + TRIANGLE_WIDTH, y_pos - TRIANGLE_HEIGHT / 2.0],
                    util::invert_color(mid_color),
                )
                .build();
            ui.set_cursor_screen_pos([x_pos, y_pos - TRIANGLE_HEIGHT / 2.0]);
            ui.invisible_button(format!("set_mid"), [TRIANGLE_WIDTH, TRIANGLE_HEIGHT]);
            let mut hovered_or_moving = false;
            if ui.is_item_hovered() {
                hovered_or_moving = true;
                ui.set_mouse_cursor(Some(MouseCursor::ResizeNS));
                if ui.is_mouse_clicked(MouseButton::Left) {
                    self.lut_mid_moving = true;
                }
            }
            if self.lut_mid_moving {
                hovered_or_moving = true;
                ui.set_mouse_cursor(Some(MouseCursor::ResizeNS));
                let [_, mouse_y] = ui.io().mouse_pos;
                let max_y_pos = pos[1] + size[1] * (1.0 - lims.2);
                let min_y_pos = pos[1] + size[1] * (1.0 - lims.0);
                let mid = (mouse_y - min_y_pos) / (max_y_pos - min_y_pos);
                self.lut.set_mid(mid);
                changed = true;
            }
            if hovered_or_moving {
                let p = self.lut.lims().1;
                let val = vmin + (vmax - vmin) * p;
                ui.tooltip_text(format!("midpoint"));
                ui.tooltip_text(format!("LIM: {:.6}", p));
                ui.tooltip_text(format!("VAL: {:.6}", val));
            }
            if !ui.is_mouse_down(MouseButton::Left) {
                self.lut_mid_moving = false;
            }

            // Max triangle
            let max_color = util::to_u32_color(self.lut.color_at(lims.2));
            let y_pos = pos[1] + size[1] * (1.0 - lims.2);
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
            if lims.2 < 1.0 {
                let max_threshold = util::lerp(vmin, vmax, lims.2);
                draw_list.add_text(
                    [x_pos + TRIANGLE_WIDTH + LABEL_HORIZONTAL_PADDING, y_pos],
                    COLOR,
                    &format!("{:.2}", max_threshold),
                );
            }
            ui.set_cursor_screen_pos([x_pos, y_pos - TRIANGLE_HEIGHT / 2.0]);
            ui.invisible_button(format!("set_max"), [TRIANGLE_WIDTH, TRIANGLE_HEIGHT]);
            let mut hovered_or_moving = false;
            if ui.is_item_hovered() {
                hovered_or_moving = true;
                ui.set_mouse_cursor(Some(MouseCursor::ResizeNS));
                if ui.is_mouse_clicked(MouseButton::Left) {
                    self.lut_max_moving = true;
                }
            }
            if self.lut_max_moving {
                hovered_or_moving = true;
                ui.set_mouse_cursor(Some(MouseCursor::ResizeNS));
                let [_, mouse_y] = ui.io().mouse_pos;
                let max = 1.0 - (mouse_y - pos[1]) / size[1];
                self.lut.set_max(max);
                changed = true;
            }
            if hovered_or_moving {
                let p = self.lut.lims().2;
                let val = vmin + (vmax - vmin) * p;
                ui.tooltip_text(format!("highlight"));
                ui.tooltip_text(format!("LIM: {:.6}", p));
                ui.tooltip_text(format!("VAL: {:.6}", val));
            }
            if !ui.is_mouse_down(MouseButton::Left) {
                self.lut_max_moving = false;
            }
        }

        let x_pos = pos[0] + 5.0;
        if self.lut.lims().1 == 0.5 {
            // Linear transfer function when mid_tone = 0.5
            for ((v1, c1), (v2, c2)) in self.lut.bounds() {
                let bottom_col = util::to_u32_color(c1);
                let top_col = util::to_u32_color(c2);
                let bottom_y_pos = pos[1] + size[1] * (1.0 - v1);
                let top_y_pos = pos[1] + size[1] * (1.0 - v2);
                draw_list.add_rect_filled_multicolor(
                    [x_pos, top_y_pos],
                    [x_pos + size[0], bottom_y_pos],
                    top_col,
                    top_col,
                    bottom_col,
                    bottom_col,
                );
            }
        } else {
            // Non-linear transfer function (Midtone Transfer Function, MTF) is adopted when mid_tone != 0.5, so we can NOT use bounds()
            let num_of_rects = 256;
            for i in 0..num_of_rects {
                let c1 = self.lut.color_at(i as f32 / num_of_rects as f32);
                let c2 = self.lut.color_at((i as f32 + 1.0) / num_of_rects as f32);
                let bottom_y_pos = pos[1] + size[1] * (1.0 - i as f32 / num_of_rects as f32);
                let top_y_pos = pos[1] + size[1] * (1.0 - (i as f32 + 1.0) / num_of_rects as f32);
                let bottom_col = util::to_u32_color(c1);
                let top_col = util::to_u32_color(c2);
                draw_list.add_rect_filled_multicolor(
                    [x_pos, top_y_pos],
                    [x_pos + size[0], bottom_y_pos],
                    top_col,
                    top_col,
                    bottom_col,
                    bottom_col,
                );
            }
        }
        let mut i = 1.0;
        let text_height = ui.text_line_height_with_spacing();
        const LABEL_HORIZONTAL_PADDING: f32 = 2.0;
        const COLOR: u32 = 0xFFFF_FFFF;
        const TICK_SIZE: f32 = 3.0;
        const TICK_COUNT: usize = 10;
        const TICK_STEP: f32 = 1.0 / TICK_COUNT as f32;
        while i >= -0.01 {
            let tick_y_pos = util::lerp(pos[1], pos[1] + size[1], i);
            let y_pos = tick_y_pos - text_height / 2.5;
            let val = vmax + (vmin - vmax) * i;
            draw_list.add_text(
                [x_pos + size[0] + LABEL_HORIZONTAL_PADDING, y_pos],
                COLOR,
                &format!("{:.2}", val),
            );
            draw_list
                .add_line(
                    [x_pos + size[0] - TICK_SIZE, tick_y_pos],
                    [x_pos + size[0], tick_y_pos],
                    COLOR,
                )
                .build();
            // TODO: Make step editable
            i -= TICK_STEP;
        }

        for (id, interaction) in self.interactions.iter_mut() {
            let stack = ui.push_id(id.id());
            match interaction {
                Interaction::Lims(Lims { lims }) => {
                    let now_lims = self.lut.lims();
                    let now_lims = [now_lims.0, now_lims.1, now_lims.2];
                    if changed {
                        *lims = now_lims;
                    } else if *lims != now_lims {
                        self.lut.set_lims(lims[0], lims[1], lims[2]);
                        changed = true;
                    }
                }
                _ => {}
            }

            stack.pop();
        }

        changed
    }

    pub(crate) fn show_bar_rgb(&mut self, ui: &Ui, pos: [f32; 2], size: [f32; 2]) -> bool {
        let mut changed = false;

        for channel in 0..3 {
            ui.set_cursor_screen_pos([pos[0] + 5.0 + (20.0 + size[0]) * channel as f32, pos[1]]);
            ui.button_with_size(format!("image_bar"), size);
            if ui.is_item_hovered() && ui.is_mouse_clicked(MouseButton::Right) {
                ui.open_popup(format!("swap-lut"));
            }
        }
        ui.popup(format!("swap-lut"), || {
            ui.text("Stretch");
            ui.separator();
            if MenuItem::new("Auto (RGB Linked)").build(ui) {
                let meds = self.image.vmed_rgb();
                let mads = self.image.vmad_rgb();
                let mut lut_min = 0.0;
                let mut lut_mid = 0.0;
                for c in 0..3 {
                    if 1.0 + mads[c] != 1.0 {
                        lut_min += meds[c] + -2.8 * mads[c];
                    }
                    lut_mid += meds[c];
                }
                let lut_min = util::clamp(lut_min / 3.0, 0.0, 1.0);
                let lut_mid = self.lut.mtf(0.25, lut_mid / 3.0 - lut_min);
                for c in 0..3 {
                    self.lut_color[c].set_lims(lut_min, lut_mid, 1.0);
                }
                changed = true;
            }
            if MenuItem::new("Auto (RGB Unlinked)").build(ui) {
                let meds = self.image.vmed_rgb();
                let mads = self.image.vmad_rgb();
                for c in 0..3 {
                    let lut_min = if 1.0 + mads[c] != 1.0 {
                        util::clamp(meds[c] + -2.8 * mads[c], 0.0, 1.0)
                    } else {
                        0.0
                    };
                    let lut_mid = self.lut.mtf(0.25, meds[c] - lut_min);
                    self.lut_color[c].set_lims(lut_min, lut_mid, 1.0);
                }
                changed = true;
            }
            if MenuItem::new("Reset").build(ui) {
                for c in 0..3 {
                    self.lut_color[c].set_lims(0.0, 0.5, 1.0);
                }
                changed = true;
            }
            if let Some(menu) = ui.begin_menu_with_enabled(format!("Lims"), true) {
                if MenuItem::new(format!("to main editor")).build(ui) {
                    let lims = [
                        [
                            self.lut_color[0].lims().0,
                            self.lut_color[0].lims().1,
                            self.lut_color[0].lims().2,
                        ],
                        [
                            self.lut_color[1].lims().0,
                            self.lut_color[1].lims().1,
                            self.lut_color[1].lims().2,
                        ],
                        [
                            self.lut_color[2].lims().0,
                            self.lut_color[2].lims().1,
                            self.lut_color[2].lims().2,
                        ],
                    ];
                    let new = Interaction::ColorLims(ColorLims::new(lims));
                    self.interactions.insert(new);
                }
                menu.end();
            }
        });
        ui.set_cursor_screen_pos([pos[0] + 5.0, pos[1]]);
        let draw_list = ui.get_window_draw_list();

        let vmin = self.image.vmin();
        let vmax = self.image.vmax();
        let vmin_rgb = self.image.vmin_rgb();
        let vmax_rgb = self.image.vmax_rgb();
        // Show triangle to change contrast
        for channels in 0..3 {
            const TRIANGLE_HEIGHT: f32 = 20.0;
            const TRIANGLE_WIDTH: f32 = 15.0;
            let lims = self.lut_color[channels].lims();
            let vmin = vmin_rgb[channels];
            let vmax = vmax_rgb[channels];
            let channel_name = match channels {
                0 => "RED",
                1 => "GREEN",
                2 => "BLUE",
                _ => "",
            };
            // Min triangle
            let min_color = util::to_u32_color(self.lut_color[channels].color_at(lims.0));
            let x_pos = pos[0] + 5.0 + 20.0 + 40.0 * channels as f32;
            let y_pos = pos[1] + size[1] * (1.0 - lims.0);
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
            ui.invisible_button(
                format!("set_min_{}", channels),
                [TRIANGLE_WIDTH, TRIANGLE_HEIGHT],
            );
            let mut hovered_or_moving = false;
            if ui.is_item_hovered() {
                hovered_or_moving = true;
                ui.set_mouse_cursor(Some(MouseCursor::ResizeNS));
                if ui.is_mouse_clicked(MouseButton::Left) {
                    self.lut_min_moving_rgb[channels] = true;
                }
            }
            if self.lut_min_moving_rgb[channels] {
                hovered_or_moving = true;
                ui.set_mouse_cursor(Some(MouseCursor::ResizeNS));
                let [_, mouse_y] = ui.io().mouse_pos;
                let min = 1.0 - (mouse_y - pos[1]) / size[1];
                self.lut_color[channels].set_min(min);
                changed = true;
            }
            if hovered_or_moving {
                let p = self.lut_color[channels].lims().0;
                let val = vmin + (vmax - vmin) * p;
                ui.tooltip_text(format!("{}, shadow", channel_name));
                ui.tooltip_text(format!("LIM: {:.6}", p));
                ui.tooltip_text(format!("VAL: {:.6}", val));
            }
            if !ui.is_mouse_down(MouseButton::Left) {
                self.lut_min_moving_rgb[channels] = false;
            }

            // Mid_tone triangle
            let mid_color = util::to_u32_color(
                self.lut_color[channels].color_at(lims.0 + (lims.2 - lims.0) * lims.1),
            );
            let y_pos = pos[1] + size[1] * (1.0 - (lims.0 + (lims.2 - lims.0) * lims.1));
            draw_list
                .add_triangle(
                    [x_pos, y_pos],
                    [x_pos + TRIANGLE_WIDTH, y_pos + TRIANGLE_HEIGHT / 2.0],
                    [x_pos + TRIANGLE_WIDTH, y_pos - TRIANGLE_HEIGHT / 2.0],
                    mid_color,
                )
                .filled(true)
                .build();
            draw_list
                .add_triangle(
                    [x_pos, y_pos],
                    [x_pos + TRIANGLE_WIDTH, y_pos + TRIANGLE_HEIGHT / 2.0],
                    [x_pos + TRIANGLE_WIDTH, y_pos - TRIANGLE_HEIGHT / 2.0],
                    util::invert_color(mid_color),
                )
                .build();
            if lims.1 != 0.5 {
                draw_list.add_text(
                    [x_pos + TRIANGLE_WIDTH + LABEL_HORIZONTAL_PADDING, y_pos],
                    COLOR,
                    &format!("midtone:{:.2}", lims.1),
                );
            }
            ui.set_cursor_screen_pos([x_pos, y_pos - TRIANGLE_HEIGHT / 2.0]);
            ui.invisible_button(
                format!("set_mid_{}", channels),
                [TRIANGLE_WIDTH, TRIANGLE_HEIGHT],
            );
            let mut hovered_or_moving = false;
            if ui.is_item_hovered() {
                hovered_or_moving = true;
                ui.set_mouse_cursor(Some(MouseCursor::ResizeNS));
                if ui.is_mouse_clicked(MouseButton::Left) {
                    self.lut_mid_moving_rgb[channels] = true;
                }
            }
            if self.lut_mid_moving_rgb[channels] {
                hovered_or_moving = true;
                ui.set_mouse_cursor(Some(MouseCursor::ResizeNS));
                let [_, mouse_y] = ui.io().mouse_pos;
                let max_y_pos = pos[1] + size[1] * (1.0 - lims.2);
                let min_y_pos = pos[1] + size[1] * (1.0 - lims.0);
                let mid = (mouse_y - min_y_pos) / (max_y_pos - min_y_pos);
                self.lut_color[channels].set_mid(mid);
                changed = true;
            }
            if hovered_or_moving {
                let vmin_lim = self.lut_color[channels].lims().0;
                let vmin = vmin + (vmax - vmin) * vmin_lim;
                let vmax_lim = self.lut_color[channels].lims().2;
                let vmax = vmin + (vmax - vmin) * vmax_lim;
                let p = self.lut_color[channels].lims().1;
                let val = vmin + (vmax - vmin) * p;
                ui.tooltip_text(format!("{}, midpoint", channel_name));
                ui.tooltip_text(format!("LIM: {:.6}", p));
                ui.tooltip_text(format!("VAL: {:.6}", val));
            }
            if !ui.is_mouse_down(MouseButton::Left) {
                self.lut_mid_moving_rgb[channels] = false;
            }

            // Max triangle
            let max_color = util::to_u32_color(self.lut_color[channels].color_at(lims.2));
            let y_pos = pos[1] + size[1] * (1.0 - lims.2);
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
            if lims.2 < 1.0 {
                let max_threshold = util::lerp(vmin, vmax, lims.2);
                draw_list.add_text(
                    [x_pos + TRIANGLE_WIDTH + LABEL_HORIZONTAL_PADDING, y_pos],
                    COLOR,
                    &format!("{:.2}", max_threshold),
                );
            }
            ui.set_cursor_screen_pos([x_pos, y_pos - TRIANGLE_HEIGHT / 2.0]);
            ui.invisible_button(
                format!("set_max_{}", channels),
                [TRIANGLE_WIDTH, TRIANGLE_HEIGHT],
            );
            let mut hovered_or_moving = false;
            if ui.is_item_hovered() {
                hovered_or_moving = true;
                ui.set_mouse_cursor(Some(MouseCursor::ResizeNS));
                if ui.is_mouse_clicked(MouseButton::Left) {
                    self.lut_max_moving_rgb[channels] = true;
                }
            }
            if self.lut_max_moving_rgb[channels] {
                hovered_or_moving = true;
                ui.set_mouse_cursor(Some(MouseCursor::ResizeNS));
                let [_, mouse_y] = ui.io().mouse_pos;
                let max = 1.0 - (mouse_y - pos[1]) / size[1];
                self.lut_color[channels].set_max(max);
                changed = true;
            }
            if hovered_or_moving {
                let p = self.lut_color[channels].lims().2;
                let val = vmin + (vmax - vmin) * p;
                ui.tooltip_text(format!("{}, highlight", channel_name));
                ui.tooltip_text(format!("LIM: {:.6}", p));
                ui.tooltip_text(format!("VAL: {:.6}", val));
            }
            if !ui.is_mouse_down(MouseButton::Left) {
                self.lut_max_moving_rgb[channels] = false;
            }
        }

        let x_pos = pos[0] + 5.0;
        for channel in 0..3 {
            let x_pos = x_pos + (size[0] + 15.0 + 5.0) * channel as f32;
            if self.lut.lims().1 == 0.5 {
                // Linear transfer function when mid_tone = 0.5
                for ((v1, c1), (v2, c2)) in self.lut_color[channel].bounds() {
                    let bottom_col = util::to_u32_color(c1);
                    let top_col = util::to_u32_color(c2);
                    let bottom_y_pos = pos[1] + size[1] * (1.0 - v1);
                    let top_y_pos = pos[1] + size[1] * (1.0 - v2);
                    draw_list.add_rect_filled_multicolor(
                        [x_pos, top_y_pos],
                        [x_pos + size[0], bottom_y_pos],
                        top_col,
                        top_col,
                        bottom_col,
                        bottom_col,
                    );
                }
            } else {
                // Non-linear transfer function (Midtone Transfer Function, MTF) is adopted when mid_tone != 0.5, so we can NOT use bounds()
                let num_of_rects = 256;
                for i in 0..num_of_rects {
                    let c1 = self.lut_color[channel].color_at(i as f32 / num_of_rects as f32);
                    let c2 =
                        self.lut_color[channel].color_at((i as f32 + 1.0) / num_of_rects as f32);
                    let bottom_y_pos = pos[1] + size[1] * (1.0 - i as f32 / num_of_rects as f32);
                    let top_y_pos =
                        pos[1] + size[1] * (1.0 - (i as f32 + 1.0) / num_of_rects as f32);
                    let bottom_col = util::to_u32_color(c1);
                    let top_col = util::to_u32_color(c2);
                    draw_list.add_rect_filled_multicolor(
                        [x_pos, top_y_pos],
                        [x_pos + size[0], bottom_y_pos],
                        top_col,
                        top_col,
                        bottom_col,
                        bottom_col,
                    );
                }
            }
        }
        let mut i = 1.0;
        let text_height = ui.text_line_height_with_spacing();
        const LABEL_HORIZONTAL_PADDING: f32 = 2.0;
        const COLOR: u32 = 0xFFFF_FFFF;
        const TICK_SIZE: f32 = 3.0;
        const TICK_COUNT: usize = 10;
        const TICK_STEP: f32 = 1.0 / TICK_COUNT as f32;
        while i >= -0.01 {
            let tick_y_pos = util::lerp(pos[1], pos[1] + size[1], i);
            let y_pos = tick_y_pos - text_height / 2.5;
            let val = vmax + (vmin - vmax) * i;
            draw_list.add_text(
                [x_pos + size[0] + LABEL_HORIZONTAL_PADDING, y_pos],
                COLOR,
                &format!("{:.2}", val),
            );
            draw_list
                .add_line(
                    [x_pos + size[0] - TICK_SIZE, tick_y_pos],
                    [x_pos + size[0], tick_y_pos],
                    COLOR,
                )
                .build();
            // TODO: Make step editable
            i -= TICK_STEP;
        }

        for (id, interaction) in self.interactions.iter_mut() {
            let stack = ui.push_id(id.id());
            match interaction {
                Interaction::ColorLims(ColorLims { lims }) => {
                    let self_lims = [
                        [
                            self.lut_color[0].lims().0,
                            self.lut_color[0].lims().1,
                            self.lut_color[0].lims().2,
                        ],
                        [
                            self.lut_color[1].lims().0,
                            self.lut_color[1].lims().1,
                            self.lut_color[1].lims().2,
                        ],
                        [
                            self.lut_color[2].lims().0,
                            self.lut_color[2].lims().1,
                            self.lut_color[2].lims().2,
                        ],
                    ];
                    if changed {
                        *lims = self_lims;
                    } else if *lims != self_lims {
                        for c in 0..3 {
                            self.lut_color[c].set_lims(lims[c][0], lims[c][1], lims[c][2]);
                        }
                        changed = true;
                    }
                }
                _ => {}
            }

            stack.pop();
        }

        changed
    }
    pub(crate) fn show_image<FX, FY>(
        &mut self,
        ui: &Ui,
        texture_id: TextureId,
        vunit: &str,
        xaxis: Option<&AxisTransform<FX>>,
        yaxis: Option<&AxisTransform<FY>>,
        max_size: (f32, f32),
        copying: &mut Option<(InteractionId, TransformIdx)>,
        store: &mut EditableValues,
        attaching: &mut Option<(OutputId, TransformIdx, usize)>,
        outputid: OutputId,
        node_editor: &AflakNodeEditor,
    ) -> Result<([[f32; 2]; 2], f32), Error>
    where
        FX: Fn(f32) -> f32,
        FY: Fn(f32) -> f32,
    {
        const IMAGE_TOP_PADDING: f32 = 10.0;

        let tex_size = self.image.tex_size();
        let (x_use_ms_for_degrees, x_relative_to_center_for_degrees) = if let Some(xaxis) = xaxis {
            if xaxis.unit() == "deg" || xaxis.unit() == "degree" {
                (
                    self.use_ms_for_degrees.0,
                    self.relative_to_center_for_degrees.0,
                )
            } else {
                (false, false)
            }
        } else {
            (false, false)
        };
        let (y_use_ms_for_degrees, y_relative_to_center_for_degrees) = if let Some(yaxis) = yaxis {
            if yaxis.unit() == "deg" || yaxis.unit() == "degree" {
                (
                    self.use_ms_for_degrees.1,
                    self.relative_to_center_for_degrees.1,
                )
            } else {
                (false, false)
            }
        } else {
            (false, false)
        };
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
            let zoom = if self.zoomkind[0] == true {
                (available_size.0 / original_size.0).min(available_size.1 / original_size.1) - 0.01
            } else if self.zoomkind[1] == true {
                0.5
            } else if self.zoomkind[2] == true {
                1.0
            } else {
                2.0
            };
            [original_size.0 * zoom, original_size.1 * zoom]
        };

        let mut s = [0.0, 0.0];
        let childwindow_size = [
            size[0] + y_labels_width * 2.0,
            size[1] + x_labels_height + IMAGE_TOP_PADDING + 25.0,
        ];
        ChildWindow::new("scrolling_region")
            .size(childwindow_size)
            .border(false)
            .scroll_bar(false)
            .movable(false)
            .scrollable(false)
            .horizontal_scrollbar(false)
            .build(ui, || {
                let draw_list = ui.get_window_draw_list();
                let p = ui.cursor_screen_pos();
                if !self.zoomkind[0] == true {
                    self.offset[0] -= self.scrolling[0];
                    self.offset[1] -= self.scrolling[1];
                    if p != self.parent_offset {
                        let delta = [self.parent_offset[0] - p[0], self.parent_offset[1] - p[1]];
                        ui.set_cursor_screen_pos([
                            self.offset[0] - delta[0],
                            self.offset[1] - delta[1],
                        ]);
                    } else {
                        self.parent_offset = p;
                        ui.set_cursor_screen_pos(self.offset);
                    }
                } else {
                    self.parent_offset = p;
                    self.offset = ui.cursor_screen_pos();
                }
                let p = ui.cursor_screen_pos();
                ui.set_cursor_screen_pos([p[0] + y_labels_width, p[1] + IMAGE_TOP_PADDING]);
                let p = ui.cursor_screen_pos();
                s = p;
                Image::new(texture_id, size).build(ui);
                ticks.draw(&draw_list, p, size);
                const MIN_WIDTH: f32 = 100.0;
                const MIN_HEIGHT: f32 = 100.0;
                let available_size = (
                    MIN_WIDTH.max(max_size.0 - y_labels_width),
                    MIN_HEIGHT.max(max_size.1 - x_labels_height - IMAGE_TOP_PADDING),
                );
                if (available_size.0 < size[0] || available_size.1 < size[1])
                    && (ui.io().key_ctrl || ui.io().key_alt)
                    && ui.is_mouse_dragging(MouseButton::Middle)
                {
                    ui.set_mouse_cursor(Some(MouseCursor::ResizeAll));
                    let mouse_delta = ui.io().mouse_delta;
                    let delta = [0.0 - mouse_delta[0], 0.0 - mouse_delta[1]];
                    self.scrolling = delta;
                } else {
                    self.scrolling = [0.0, 0.0];
                }
                let is_image_hovered = ui.is_item_hovered();

                let abs_mouse_pos = ui.io().mouse_pos;
                let mouse_pos = (abs_mouse_pos[0] - p[0], -abs_mouse_pos[1] + p[1] + size[1]);
                self.mouse_pos = (
                    mouse_pos.0 / size[0] * tex_size.0 as f32,
                    mouse_pos.1 / size[1] * tex_size.1 as f32,
                );

                if is_image_hovered {
                    let x = self.mouse_pos.0 as usize;
                    let y = self.mouse_pos.1 as usize;
                    match self.image.ndim() {
                        2 => {
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
                                    if ui.io().key_shift {
                                        ui.tooltip(|| {
                                            ui.text(text);
                                            let x = self.mouse_pos.0;
                                            let y = self.mouse_pos.1;
                                            let x1 = (x - 10.0) as f32 / tex_size.0;
                                            let x2 = (x + 11.0) as f32 / tex_size.0;
                                            let y1 = (tex_size.1 - (y + 10.0) as f32) / tex_size.1;
                                            let y2 = (tex_size.1 - (y - 11.0) as f32) / tex_size.1;
                                            let x1 = util::clamp(x1, 0.0, 1.0);
                                            let x2 = util::clamp(x2, 0.0, 1.0);
                                            let y1 = util::clamp(y1, 0.0, 1.0);
                                            let y2 = util::clamp(y2, 0.0, 1.0);
                                            Image::new(texture_id, [300.0, 300.0])
                                                .uv0([x1, y1])
                                                .uv1([x2, y2])
                                                .build(ui);
                                        });
                                    } else {
                                        ui.tooltip_text(text);
                                    }
                                }
                            }
                        }
                        3 => {
                            if y < self.image.dim().1 {
                                let rindex = [0, self.image.dim().1 - 1 - y, x];
                                let gindex = [1, self.image.dim().1 - 1 - y, x];
                                let bindex = [2, self.image.dim().1 - 1 - y, x];
                                let x_measurement = xaxis.map(|axis| Measurement {
                                    v: axis.pix2world(x as f32),
                                    unit: axis.unit(),
                                });
                                let y_measurement = yaxis.map(|axis| Measurement {
                                    v: axis.pix2world(y as f32),
                                    unit: axis.unit(),
                                });
                                if let (Some(rval), Some(gval), Some(bval)) = (
                                    self.image.get_color(rindex),
                                    self.image.get_color(gindex),
                                    self.image.get_color(bindex),
                                ) {
                                    let text = self.make_tooltip_for_color(
                                        (x, y),
                                        x_measurement,
                                        y_measurement,
                                        Measurement {
                                            v: rval,
                                            unit: vunit,
                                        },
                                        Measurement {
                                            v: gval,
                                            unit: vunit,
                                        },
                                        Measurement {
                                            v: bval,
                                            unit: vunit,
                                        },
                                    );
                                    if ui.io().key_shift {
                                        ui.tooltip(|| {
                                            ui.text(text);
                                            let x = self.mouse_pos.0;
                                            let y = self.mouse_pos.1;
                                            let x1 = (x - 10.0) as f32 / tex_size.0;
                                            let x2 = (x + 11.0) as f32 / tex_size.0;
                                            let y1 = (tex_size.1 - (y + 10.0) as f32) / tex_size.1;
                                            let y2 = (tex_size.1 - (y - 11.0) as f32) / tex_size.1;
                                            let x1 = util::clamp(x1, 0.0, 1.0);
                                            let x2 = util::clamp(x2, 0.0, 1.0);
                                            let y1 = util::clamp(y1, 0.0, 1.0);
                                            let y2 = util::clamp(y2, 0.0, 1.0);
                                            Image::new(texture_id, [300.0, 300.0])
                                                .uv0([x1, y1])
                                                .uv1([x2, y2])
                                                .build(ui);
                                        });
                                    } else {
                                        ui.tooltip_text(text);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }

                    if ui.is_mouse_clicked(MouseButton::Right) {
                        ui.open_popup(format!("add-interaction-handle"))
                    }
                }

                if self.show_approx_line {
                    let mut maxpoints = Vec::<(usize, usize)>::new();
                    for i in 0..self.image.dim().1 {
                        let mut maxv = std::f32::MIN;
                        let mut maxy = 0;
                        for j in 0..self.image.dim().0 {
                            let index = [self.image.dim().0 - 1 - j, i];
                            if let Some(val) = self.image.get(index) {
                                if maxv < val {
                                    maxy = j;
                                    maxv = val;
                                }
                            }
                        }
                        maxpoints.push((i, maxy));
                        let x0 = p[0] + (i as f32) / tex_size.0 as f32 * size[0];
                        let y0 = p[1] + size[1] - ((maxy + 1) as f32) / tex_size.1 as f32 * size[1];
                        draw_list
                            .add_rect(
                                [x0, y0],
                                [x0 + size[0] / tex_size.0, y0 + size[1] / tex_size.1],
                                0x8000_00FF,
                            )
                            .filled(true)
                            .build();
                    }
                    let mut sigma_xy: isize = 0;
                    let mut sigma_x: isize = 0;
                    let mut sigma_y: isize = 0;
                    let mut sigma_xx: isize = 0;
                    let n: isize = maxpoints.len() as isize;
                    for (x, y) in maxpoints {
                        sigma_xy += (x * y) as isize;
                        sigma_x += x as isize;
                        sigma_y += y as isize;
                        sigma_xx += (x * x) as isize;
                    }
                    let slope = ((n * sigma_xy - sigma_x * sigma_y) as f32
                        / (n * sigma_xx - sigma_x * sigma_x) as f32)
                        as f32;
                    let y_intercept = ((sigma_xx * sigma_y - sigma_xy * sigma_x) as f32
                        / (n * sigma_xx - sigma_x * sigma_x) as f32)
                        as f32;
                    let line_x0 = p[0];
                    let line_y0 = p[1] + size[1] - y_intercept / tex_size.1 as f32 * size[1];
                    let line_x1 = p[0] + n as f32 / tex_size.0 as f32 * size[0];
                    let line_y1 = p[1] + size[1]
                        - (slope * n as f32 + y_intercept as f32) / tex_size.1 as f32 * size[1];
                    draw_list
                        .add_line([line_x0, line_y0], [line_x1, line_y1], 0x8000_00FF)
                        .build();
                }

                if self.show_axis_option {
                    Window::new(&ImString::new(format!("Axes option of {:?}", outputid)))
                        .size([300.0, 200.0], Condition::Appearing)
                        .resizable(false)
                        .build(ui, || match (xaxis, yaxis) {
                            (Some(xaxis), Some(yaxis)) => {
                                let mut available = false;
                                ui.text(format!("XAxis: {} ({})", xaxis.label(), xaxis.unit()));
                                if xaxis.unit() == "deg" || xaxis.unit() == "degree" {
                                    available = true;
                                    ui.checkbox(
                                        "Use {hours}h {minutes}m {seconds}s##XAxis",
                                        &mut self.use_ms_for_degrees.0,
                                    );
                                    ui.checkbox(
                                        "Display relative to the center##XAxis",
                                        &mut self.relative_to_center_for_degrees.0,
                                    );
                                }
                                ui.text(format!("YAxis: {} ({})", yaxis.label(), yaxis.unit()));
                                if yaxis.unit() == "deg" || yaxis.unit() == "degree" {
                                    available = true;
                                    ui.checkbox(
                                        "Use {degrees}° {minutes}' {seconds}''##YAxis",
                                        &mut self.use_ms_for_degrees.1,
                                    );
                                    ui.checkbox(
                                        "Display relative to the center###YAxis",
                                        &mut self.relative_to_center_for_degrees.1,
                                    );
                                }
                                if !available {
                                    ui.text("NO available options.");
                                }
                            }
                            (Some(xaxis), None) => {
                                let mut available = false;
                                ui.text(format!("XAxis: {} ({})", xaxis.label(), xaxis.unit()));
                                if xaxis.unit() == "deg" || xaxis.unit() == "degree" {
                                    available = true;
                                    ui.checkbox(
                                        "Use minutes & seconds",
                                        &mut self.use_ms_for_degrees.0,
                                    );
                                    ui.checkbox(
                                        "Displayed relative to the center",
                                        &mut self.relative_to_center_for_degrees.0,
                                    );
                                }
                                if !available {
                                    ui.text("NO available options.");
                                }
                            }
                            (None, Some(yaxis)) => {
                                let mut available = false;
                                ui.text(format!("YAxis: {} ({})", yaxis.label(), yaxis.unit()));
                                if yaxis.unit() == "deg" || yaxis.unit() == "degree" {
                                    available = true;
                                    ui.checkbox(
                                        "Use minutes & seconds",
                                        &mut self.use_ms_for_degrees.1,
                                    );
                                    ui.checkbox(
                                        "Displayed relative to the center",
                                        &mut self.relative_to_center_for_degrees.1,
                                    );
                                }
                                if !available {
                                    ui.text("NO available options.");
                                }
                            }
                            (None, None) => {
                                ui.text("NO available options.");
                            }
                        });
                }

                // Add interaction handlers
                ui.popup(format!("add-interaction-handle"), || {
                    ui.text("Add interaction handle");
                    ui.separator();
                    if let Some(menu) = ui.begin_menu_with_enabled(format!("Horizontal Line"), true)
                    {
                        if MenuItem::new(format!("to main editor")).build(ui) {
                            let new = Interaction::HorizontalLine(HorizontalLine::new(
                                self.mouse_pos.1.round(),
                            ));
                            self.interactions.insert(new);
                        }
                        for macr in node_editor.macros.macros() {
                            if MenuItem::new(&format!("to macro: {}", macr.name())).build(ui) {
                                let new = Interaction::HorizontalLine(HorizontalLine::new(
                                    self.mouse_pos.1.round(),
                                ));
                                self.interactions.insert(new);
                                let macro_id = macr.id();
                                let mut dstw = macr.write();
                                let t_idx = dstw.dst_mut().add_owned_transform(
                                    Transform::new_constant(aflak_primitives::IOValue::Float(
                                        self.mouse_pos.1.round(),
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
                    if let Some(menu) = ui.begin_menu_with_enabled(format!("Vertical Line"), true) {
                        if MenuItem::new(format!("to main editor")).build(ui) {
                            let new = Interaction::VerticalLine(VerticalLine::new(
                                self.mouse_pos.0.round(),
                            ));
                            self.interactions.insert(new);
                        }
                        for macr in node_editor.macros.macros() {
                            if MenuItem::new(&format!("to macro: {}", macr.name())).build(ui) {
                                let new = Interaction::VerticalLine(VerticalLine::new(
                                    self.mouse_pos.0.round(),
                                ));
                                self.interactions.insert(new);
                                let macro_id = macr.id();
                                let mut dstw = macr.write();
                                let t_idx = dstw.dst_mut().add_owned_transform(
                                    Transform::new_constant(aflak_primitives::IOValue::Float(
                                        self.mouse_pos.0.round(),
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
                    if MenuItem::new(format!("Region of interest")).build(ui) {
                        let new = Interaction::FinedGrainedROI(FinedGrainedROI::new(
                            self.roi_input.gen_id(),
                        ));
                        self.interactions.insert(new);
                    }
                    if MenuItem::new(format!("Line")).build(ui) {
                        let new = Interaction::Line(Line::new());
                        self.interactions.insert(new);
                    }
                    if MenuItem::new(format!("Circle")).build(ui) {
                        let new = Interaction::Circle(Circle::new(self.circle_input.gen_id()));
                        self.interactions.insert(new);
                    }
                    if let Some((_, t_idx)) = *copying {
                        ui.separator();
                        ui.text("Paste Line Options");
                        ui.separator();
                        if MenuItem::new(format!("Paste Line as Horizontal Line")).build(ui) {
                            let new = Interaction::HorizontalLine(HorizontalLine::new(
                                self.mouse_pos.1.round(),
                            ));
                            self.interactions.insert(new);
                            store.insert(self.interactions.id(), t_idx);
                            *copying = None;
                        }
                        if MenuItem::new(format!("Paste Line as Vertical Line")).build(ui) {
                            let new = Interaction::VerticalLine(VerticalLine::new(
                                self.mouse_pos.0.round(),
                            ));
                            self.interactions.insert(new);
                            store.insert(self.interactions.id(), t_idx);
                            *copying = None;
                        }
                    }
                });
                if let Some((o, t_idx, kind)) = *attaching {
                    if o == outputid && (kind == 0 || kind == 1 || kind == 2) {
                        let mut already_insert = false;
                        for d in store.iter() {
                            if *d.1 == t_idx {
                                already_insert = true;
                                break;
                            }
                        }
                        if !already_insert {
                            let new = if kind == 0 {
                                Interaction::HorizontalLine(HorizontalLine::new(
                                    self.mouse_pos.1.round(),
                                ))
                            } else if kind == 1 {
                                Interaction::VerticalLine(VerticalLine::new(
                                    self.mouse_pos.0.round(),
                                ))
                            } else {
                                Interaction::FinedGrainedROI(FinedGrainedROI::new(
                                    self.roi_input.gen_id(),
                                ))
                            };
                            self.interactions.insert(new);
                            store.insert(self.interactions.id(), t_idx);
                        } else {
                            eprintln!("{:?} is already bound", t_idx)
                        }
                        *attaching = None;
                    }
                }

                let mut line_marked_for_deletion = None;
                for (id, interaction) in self.interactions.iter_mut() {
                    let stack = ui.push_id(id.id());
                    const LINE_COLOR: u32 = 0xFFFF_FFFF;
                    match interaction {
                        Interaction::HorizontalLine(HorizontalLine { height, moving }) => {
                            let x = p[0];
                            let y = p[1] + size[1] - *height / tex_size.1 as f32 * size[1];

                            const CLICKABLE_HEIGHT: f32 = 5.0;

                            ui.set_cursor_screen_pos([x, y - CLICKABLE_HEIGHT]);

                            ui.invisible_button(
                                format!("horizontal-line"),
                                [size[0], 2.0 * CLICKABLE_HEIGHT],
                            );
                            if ui.is_item_hovered() {
                                ui.set_mouse_cursor(Some(MouseCursor::ResizeNS));
                                if ui.is_mouse_clicked(MouseButton::Left) {
                                    *moving = true;
                                }
                                if ui.is_mouse_clicked(MouseButton::Right) {
                                    ui.open_popup(format!("edit-horizontal-line"))
                                }
                            }
                            if *moving {
                                *height =
                                    util::clamp(self.mouse_pos.1.round(), 0.0, tex_size.1 as f32);
                            }
                            if !ui.is_mouse_down(MouseButton::Left) {
                                *moving = false;
                            }

                            draw_list
                                .add_line([x, y], [x + size[0], y], LINE_COLOR)
                                .build();

                            ui.popup(format!("edit-horizontal-line"), || {
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
                        Interaction::VerticalLine(VerticalLine { x_pos, moving }) => {
                            let x = p[0] + *x_pos / tex_size.0 as f32 * size[0];
                            let y = p[1];

                            const CLICKABLE_WIDTH: f32 = 5.0;

                            ui.set_cursor_screen_pos([x - CLICKABLE_WIDTH, y]);
                            ui.invisible_button(
                                format!("vertical-line"),
                                [2.0 * CLICKABLE_WIDTH, size[1]],
                            );
                            if ui.is_item_hovered() {
                                ui.set_mouse_cursor(Some(MouseCursor::ResizeEW));
                                if ui.is_mouse_clicked(MouseButton::Left) {
                                    *moving = true;
                                }
                                if ui.is_mouse_clicked(MouseButton::Right) {
                                    ui.open_popup(format!("edit-vertical-line"))
                                }
                            }
                            if *moving {
                                *x_pos =
                                    util::clamp(self.mouse_pos.0.round(), 0.0, tex_size.0 as f32);
                            }
                            if !ui.is_mouse_down(MouseButton::Left) {
                                *moving = false;
                            }
                            draw_list
                                .add_line([x, y], [x, y + size[1]], LINE_COLOR)
                                .build();

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
                        Interaction::FinedGrainedROI(FinedGrainedROI {
                            id,
                            pixels,
                            changed,
                        }) => {
                            let selected = self.roi_input.is_selected(*id);

                            let pixel_size_x = size[0] / tex_size.0 as f32;
                            let pixel_size_y = size[1] / tex_size.1 as f32;
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
                                let x = p[0] + i / tex_size.0 as f32 * size[0];
                                let y = p[1] + size[1] - j / tex_size.1 as f32 * size[1];
                                draw_list.add_rect_filled_multicolor(
                                    [x, y],
                                    [x + pixel_size_x, y - pixel_size_y],
                                    roi_color,
                                    roi_color,
                                    roi_color,
                                    roi_color,
                                );
                            }

                            if selected
                                && is_image_hovered
                                && ui.is_mouse_clicked(MouseButton::Left)
                            {
                                let pixel = (self.mouse_pos.0 as usize, self.mouse_pos.1 as usize);
                                let some_position =
                                    pixels.iter().position(|&pixel_| pixel_ == pixel);
                                if let Some(position) = some_position {
                                    pixels.remove(position);
                                } else {
                                    pixels.push(pixel);
                                }
                                *changed = true;
                            } else {
                                *changed = false;
                            }
                        }
                        Interaction::Line(Line {
                            endpoints,
                            endpoints_zero,
                            endpointsfill,
                            pixels,
                            pre_mousepos,
                            allmoving,
                            edgemoving,
                            show_rotate,
                            degree,
                        }) => {
                            const CLICKABLE_WIDTH: f32 = 5.0;
                            if is_image_hovered && ui.is_mouse_clicked(MouseButton::Left) {
                                if endpointsfill.0 == false {
                                    endpointsfill.0 = true;
                                    endpoints.0 = self.mouse_pos;
                                } else if endpointsfill.1 == false {
                                    endpointsfill.1 = true;
                                    endpoints.1 = self.mouse_pos;
                                    pixels.clear();
                                    get_pixels_of_line(pixels, *endpoints, tex_size);
                                }
                            }
                            let x0 = p[0] + (endpoints.0).0 as f32 / tex_size.0 as f32 * size[0];
                            let y0 = p[1] + size[1]
                                - (endpoints.0).1 as f32 / tex_size.1 as f32 * size[1];
                            let x1 = p[0] + (endpoints.1).0 as f32 / tex_size.0 as f32 * size[0];
                            let y1 = p[1] + size[1]
                                - (endpoints.1).1 as f32 / tex_size.1 as f32 * size[1];
                            let linevec = (x1 - x0, y1 - y0);
                            let linevecsize =
                                (linevec.0 * linevec.0 + linevec.1 * linevec.1).sqrt();
                            let center = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
                            let angle = (linevec.1).atan2(linevec.0);
                            let rotated = (
                                linevec.1 / linevecsize * CLICKABLE_WIDTH,
                                -linevec.0 / linevecsize * CLICKABLE_WIDTH,
                            );
                            let upperleft = (x0 + rotated.0, y0 + rotated.1);
                            let center_to_upperleft =
                                (upperleft.0 - center.0, upperleft.1 - center.1);
                            let mousepos = (
                                p[0] + self.mouse_pos.0 as f32 / tex_size.0 as f32 * size[0],
                                p[1] + size[1]
                                    - self.mouse_pos.1 as f32 / tex_size.1 as f32 * size[1],
                            );
                            let center_to_mousepos = (mousepos.0 - center.0, mousepos.1 - center.1);
                            let rotated_upperleft = (
                                center.0
                                    + center_to_upperleft.0 * (-angle).cos()
                                    + center_to_upperleft.1 * -(-angle).sin(),
                                center.1
                                    + center_to_upperleft.0 * (-angle).sin()
                                    + center_to_upperleft.1 * (-angle).cos(),
                            );
                            let rotated_mousepos = (
                                center.0
                                    + center_to_mousepos.0 * (-angle).cos()
                                    + center_to_mousepos.1 * -(-angle).sin(),
                                center.1
                                    + center_to_mousepos.0 * (-angle).sin()
                                    + center_to_mousepos.1 * (-angle).cos(),
                            );
                            if endpointsfill.0 && endpointsfill.1 {
                                draw_list.add_line([x0, y0], [x1, y1], LINE_COLOR).build();
                                if (x0 - mousepos.0) * (x0 - mousepos.0)
                                    + (y0 - mousepos.1) * (y0 - mousepos.1)
                                    <= CLICKABLE_WIDTH * CLICKABLE_WIDTH
                                {
                                    if !edgemoving.0 {
                                        const CIRCLE_COLOR_BEGIN: u32 = 0x8000_00FF;
                                        draw_list
                                            .add_circle(
                                                [x0, y0],
                                                CLICKABLE_WIDTH * 2.0,
                                                CIRCLE_COLOR_BEGIN,
                                            )
                                            .filled(true)
                                            .build();
                                    }
                                    ui.set_cursor_screen_pos([
                                        mousepos.0 - CLICKABLE_WIDTH,
                                        mousepos.1 - CLICKABLE_WIDTH,
                                    ]);
                                    ui.invisible_button(
                                        format!("line"),
                                        [CLICKABLE_WIDTH * 2.0, CLICKABLE_WIDTH * 2.0],
                                    );
                                    ui.set_mouse_cursor(Some(MouseCursor::ResizeAll));

                                    if ui.is_mouse_clicked(MouseButton::Left) && !edgemoving.0 {
                                        edgemoving.0 = true;
                                    }
                                    if ui.is_mouse_clicked(MouseButton::Right) {
                                        ui.open_popup(format!("edit-line"))
                                    }
                                } else if (x1 - mousepos.0) * (x1 - mousepos.0)
                                    + (y1 - mousepos.1) * (y1 - mousepos.1)
                                    <= CLICKABLE_WIDTH * CLICKABLE_WIDTH
                                {
                                    if !edgemoving.1 {
                                        const CIRCLE_COLOR_END: u32 = 0x80FF_0000;
                                        draw_list
                                            .add_circle(
                                                [x1, y1],
                                                CLICKABLE_WIDTH * 2.0,
                                                CIRCLE_COLOR_END,
                                            )
                                            .filled(true)
                                            .build();
                                    }
                                    ui.set_cursor_screen_pos([
                                        mousepos.0 - CLICKABLE_WIDTH,
                                        mousepos.1 - CLICKABLE_WIDTH,
                                    ]);
                                    ui.invisible_button(
                                        format!("line"),
                                        [CLICKABLE_WIDTH * 2.0, CLICKABLE_WIDTH * 2.0],
                                    );
                                    ui.set_mouse_cursor(Some(MouseCursor::ResizeAll));

                                    if ui.is_mouse_clicked(MouseButton::Left) && !edgemoving.1 {
                                        edgemoving.1 = true;
                                    }
                                    if ui.is_mouse_clicked(MouseButton::Right) {
                                        ui.open_popup(format!("edit-line"))
                                    }
                                } else if rotated_upperleft.0 <= rotated_mousepos.0
                                    && rotated_mousepos.0 <= rotated_upperleft.0 + linevecsize
                                    && rotated_upperleft.1 <= rotated_mousepos.1
                                    && rotated_mousepos.1
                                        <= rotated_upperleft.1 + CLICKABLE_WIDTH * 2.0
                                {
                                    ui.set_cursor_screen_pos([
                                        mousepos.0 - CLICKABLE_WIDTH,
                                        mousepos.1 - CLICKABLE_WIDTH,
                                    ]);
                                    ui.invisible_button(
                                        format!("line"),
                                        [CLICKABLE_WIDTH * 2.0, CLICKABLE_WIDTH * 2.0],
                                    );
                                    ui.set_mouse_cursor(Some(MouseCursor::ResizeAll));

                                    if ui.is_mouse_clicked(MouseButton::Left) && !*allmoving {
                                        *allmoving = true;
                                        *pre_mousepos = self.mouse_pos;
                                    }
                                    if ui.is_mouse_clicked(MouseButton::Right) {
                                        ui.open_popup(format!("edit-line"))
                                    }
                                }
                                if *allmoving {
                                    let now_mousepos = self.mouse_pos;
                                    (endpoints.0).0 += now_mousepos.0 - pre_mousepos.0;
                                    (endpoints.0).1 += now_mousepos.1 - pre_mousepos.1;
                                    (endpoints.1).0 += now_mousepos.0 - pre_mousepos.0;
                                    (endpoints.1).1 += now_mousepos.1 - pre_mousepos.1;
                                    *pre_mousepos = now_mousepos;
                                    pixels.clear();
                                    get_pixels_of_line(pixels, *endpoints, tex_size);
                                } else if edgemoving.0 {
                                    const CIRCLE_COLOR_BEGIN: u32 = 0x8000_00FF;
                                    draw_list
                                        .add_circle(
                                            [x0, y0],
                                            CLICKABLE_WIDTH * 2.0,
                                            CIRCLE_COLOR_BEGIN,
                                        )
                                        .filled(true)
                                        .build();
                                    endpoints.0 = self.mouse_pos;
                                    pixels.clear();
                                    get_pixels_of_line(pixels, *endpoints, tex_size);
                                } else if edgemoving.1 {
                                    const CIRCLE_COLOR_END: u32 = 0x80FF_0000;
                                    draw_list
                                        .add_circle(
                                            [x1, y1],
                                            CLICKABLE_WIDTH * 2.0,
                                            CIRCLE_COLOR_END,
                                        )
                                        .filled(true)
                                        .build();
                                    endpoints.1 = self.mouse_pos;
                                    pixels.clear();
                                    get_pixels_of_line(pixels, *endpoints, tex_size);
                                } else if !is_image_hovered {
                                    if *allmoving {
                                        *allmoving = false;
                                    } else if edgemoving.0 {
                                        edgemoving.0 = false;
                                    } else if edgemoving.1 {
                                        edgemoving.1 = false;
                                    }
                                }
                                if edgemoving.0 {
                                    if (endpoints.0).0 > tex_size.0 {
                                        (endpoints.0).0 = tex_size.0;
                                    } else if (endpoints.0).0 < 0.0 {
                                        (endpoints.0).0 = 0.0;
                                    }
                                    if (endpoints.0).1 > tex_size.1 {
                                        (endpoints.0).1 = tex_size.1;
                                    } else if (endpoints.0).1 < 0.0 {
                                        (endpoints.0).1 = 0.0;
                                    }
                                } else if edgemoving.1 {
                                    if (endpoints.1).0 > tex_size.0 {
                                        (endpoints.1).0 = tex_size.0;
                                    } else if (endpoints.1).0 < 0.0 {
                                        (endpoints.1).0 = 0.0;
                                    }
                                    if (endpoints.1).1 > tex_size.1 {
                                        (endpoints.1).1 = tex_size.1;
                                    } else if (endpoints.1).1 < 0.0 {
                                        (endpoints.1).1 = 0.0;
                                    }
                                } else if *allmoving {
                                    if (endpoints.0).0 > tex_size.0 {
                                        *allmoving = false;
                                        (endpoints.0).0 = tex_size.0;
                                    } else if (endpoints.0).0 < 0.0 {
                                        *allmoving = false;
                                        (endpoints.0).0 = 0.0;
                                    }
                                    if (endpoints.0).1 > tex_size.1 {
                                        *allmoving = false;
                                        (endpoints.0).1 = tex_size.1;
                                    } else if (endpoints.0).1 < 0.0 {
                                        *allmoving = false;
                                        (endpoints.0).1 = 0.0;
                                    }
                                    if (endpoints.1).0 > tex_size.0 {
                                        *allmoving = false;
                                        (endpoints.1).0 = tex_size.0;
                                    } else if (endpoints.1).0 < 0.0 {
                                        *allmoving = false;
                                        (endpoints.1).0 = 0.0;
                                    }
                                    if (endpoints.1).1 > tex_size.1 {
                                        *allmoving = false;
                                        (endpoints.1).1 = tex_size.1;
                                    } else if (endpoints.1).1 < 0.0 {
                                        *allmoving = false;
                                        (endpoints.1).1 = 0.0;
                                    }
                                }
                                if !ui.is_mouse_down(MouseButton::Left) {
                                    if *allmoving {
                                        *allmoving = false;
                                    } else if edgemoving.0 {
                                        edgemoving.0 = false;
                                    } else if edgemoving.1 {
                                        edgemoving.1 = false;
                                    }
                                }

                                ui.popup(format!("edit-line"), || {
                                    if MenuItem::new(format!("Delete Line")).build(ui) {
                                        line_marked_for_deletion = Some(*id);
                                    } else if MenuItem::new(format!("Rotate Line")).build(ui) {
                                        *show_rotate = true;
                                    }
                                });
                                if *show_rotate {
                                    Window::new(&ImString::new(format!("Rotate #{:?}", id)))
                                        .size([300.0, 50.0], Condition::Appearing)
                                        .resizable(false)
                                        .build(ui, || {
                                            Slider::new(format!("Degree"), -180, 180)
                                                .build(ui, degree);
                                        });
                                    if !*allmoving && !edgemoving.0 && !edgemoving.1 {
                                        if *degree == 0 {
                                            *endpoints_zero = *endpoints;
                                        }
                                        let midpoint = (
                                            ((endpoints_zero.0).0 + (endpoints_zero.1).0) / 2.0,
                                            ((endpoints_zero.0).1 + (endpoints_zero.1).1) / 2.0,
                                        );
                                        let vector1 = (
                                            (endpoints_zero.0).0 - midpoint.0,
                                            (endpoints_zero.0).1 - midpoint.1,
                                        );
                                        let vector2 = (
                                            (endpoints_zero.1).0 - midpoint.0,
                                            (endpoints_zero.1).1 - midpoint.1,
                                        );
                                        let new_endpoint1 = (
                                            midpoint.0
                                                + vector1.0
                                                    * (*degree as f32 / 180.0
                                                        * std::f32::consts::PI)
                                                        .cos()
                                                - vector1.1
                                                    * (*degree as f32 / 180.0
                                                        * std::f32::consts::PI)
                                                        .sin(),
                                            midpoint.1
                                                + vector1.0
                                                    * (*degree as f32 / 180.0
                                                        * std::f32::consts::PI)
                                                        .sin()
                                                + vector1.1
                                                    * (*degree as f32 / 180.0
                                                        * std::f32::consts::PI)
                                                        .cos(),
                                        );
                                        let new_endpoint2 = (
                                            midpoint.0
                                                + vector2.0
                                                    * (*degree as f32 / 180.0
                                                        * std::f32::consts::PI)
                                                        .cos()
                                                - vector2.1
                                                    * (*degree as f32 / 180.0
                                                        * std::f32::consts::PI)
                                                        .sin(),
                                            midpoint.1
                                                + vector2.0
                                                    * (*degree as f32 / 180.0
                                                        * std::f32::consts::PI)
                                                        .sin()
                                                + vector2.1
                                                    * (*degree as f32 / 180.0
                                                        * std::f32::consts::PI)
                                                        .cos(),
                                        );
                                        endpoints.0 = new_endpoint1;
                                        endpoints.1 = new_endpoint2;
                                        pixels.clear();
                                        get_pixels_of_line(pixels, *endpoints, tex_size);
                                    } else {
                                        *show_rotate = false;
                                        *degree = 0;
                                    }
                                }
                            } else if endpointsfill.0 {
                                draw_list
                                    .add_line([x0, y0], [mousepos.0, mousepos.1], LINE_COLOR)
                                    .build();
                            }
                            fn get_pixels_of_line(
                                pixels: &mut Vec<(usize, usize)>,
                                endpoints: ((f32, f32), (f32, f32)),
                                tex_size: (f32, f32),
                            ) {
                                let sx = (endpoints.0).0 as isize;
                                let sy = (endpoints.0).1 as isize;
                                let dx = (endpoints.1).0 as isize;
                                let dy = (endpoints.1).1 as isize;
                                let mut x = sx;
                                let mut y = sy;
                                let wx =
                                    ((endpoints.1).0 as isize - (endpoints.0).0 as isize).abs();
                                let wy =
                                    ((endpoints.1).1 as isize - (endpoints.0).1 as isize).abs();
                                let xmode = wx >= wy;
                                let mut derr = 0;
                                while x != dx || y != dy {
                                    if 0.0 <= (x as f32)
                                        && (x as f32) < tex_size.0
                                        && 0.0 <= (y as f32)
                                        && (y as f32) < tex_size.1
                                    {
                                        pixels.push((x as usize, y as usize));
                                    }
                                    if xmode {
                                        if sx < dx {
                                            x += 1;
                                        } else {
                                            x -= 1;
                                        }
                                        derr += (dy - sy) << 1;
                                        if derr > wx {
                                            y += 1;
                                            derr -= wx << 1;
                                        } else if derr < -wx {
                                            y -= 1;
                                            derr += wx << 1;
                                        }
                                    } else {
                                        if sy < dy {
                                            y += 1;
                                        } else {
                                            y -= 1;
                                        }
                                        derr += (dx - sx) << 1;
                                        if derr > wy {
                                            x += 1;
                                            derr -= wy << 1;
                                        } else if derr < -wy {
                                            x -= 1;
                                            derr += wy << 1;
                                        }
                                    }
                                }
                            }
                        }
                        Interaction::Circle(Circle {
                            id,
                            center,
                            radius,
                            parametersfill,
                            pixels: _pixels,
                        }) => {
                            let selected = self.circle_input.is_selected(*id);
                            if selected
                                && is_image_hovered
                                && ui.is_mouse_clicked(MouseButton::Left)
                            {
                                let pixel = (self.mouse_pos.0 as usize, self.mouse_pos.1 as usize);
                                if parametersfill.0 == false {
                                    parametersfill.0 = true;
                                    *center = pixel;
                                } else if parametersfill.1 == false {
                                    parametersfill.1 = true;
                                    let x0 = p[0] + center.0 as f32 / tex_size.0 as f32 * size[0];
                                    let y0 = p[1] + size[1]
                                        - center.1 as f32 / tex_size.1 as f32 * size[1];
                                    let x1 = p[0]
                                        + self.mouse_pos.0 as f32 / tex_size.0 as f32 * size[0];
                                    let y1 = p[1] + size[1]
                                        - self.mouse_pos.1 as f32 / tex_size.1 as f32 * size[1];
                                    let rad =
                                        ((x0 - x1) * (x0 - x1) + (y0 - y1) * (y0 - y1)).sqrt();
                                    *radius = rad;
                                }
                            }

                            if parametersfill.0 && parametersfill.1 {
                                let x0 = p[0] + center.0 as f32 / tex_size.0 as f32 * size[0];
                                let y0 =
                                    p[1] + size[1] - center.1 as f32 / tex_size.1 as f32 * size[1];
                                draw_list
                                    .add_circle([x0, y0], *radius as f32, LINE_COLOR)
                                    .num_segments(50)
                                    .build();
                            } else if parametersfill.0 {
                                let x0 = p[0] + center.0 as f32 / tex_size.0 as f32 * size[0];
                                let y0 =
                                    p[1] + size[1] - center.1 as f32 / tex_size.1 as f32 * size[1];
                                let x1 =
                                    p[0] + self.mouse_pos.0 as f32 / tex_size.0 as f32 * size[0];
                                let y1 = p[1] + size[1]
                                    - self.mouse_pos.1 as f32 / tex_size.1 as f32 * size[1];
                                let rad = ((x0 - x1) * (x0 - x1) + (y0 - y1) * (y0 - y1)).sqrt();
                                draw_list
                                    .add_circle([x0, y0], rad, LINE_COLOR)
                                    .num_segments(50)
                                    .build();
                            }
                        }
                        // Used in show_bar
                        Interaction::Lims(_) => {}
                        Interaction::ColorLims(_) => {}
                    }
                    stack.pop();
                }

                if let Some(line_id) = line_marked_for_deletion {
                    self.interactions.remove(line_id);
                }
                let p = ui.cursor_screen_pos();
                ui.set_cursor_screen_pos([p[0] + y_labels_width, p[1] + 25.0]);
                self.show_roi_selector(ui);
            });
        ui.set_cursor_screen_pos(s);
        let p = ui.cursor_screen_pos();
        Ok(([p, size], x_labels_height))
    }

    pub(crate) fn show_hist(&self, ui: &Ui, pos: [f32; 2], size: [f32; 2]) {
        let vmin = self.image.vmin();
        let vmax = self.image.vmax();

        const FILL_COLOR: u32 = 0xFF99_9999;
        const BORDER_COLOR: u32 = 0xFF00_0000;
        let hist = self.image.hist();
        if let Some(max_count) = hist.iter().map(|bin| bin.count).max() {
            let draw_list = ui.get_window_draw_list();

            let x_pos = pos[0];
            for bin in hist {
                let y_pos = pos[1] + size[1] / (vmax - vmin) * (vmax - bin.start);
                let y_pos_end = pos[1] + size[1] / (vmax - vmin) * (vmax - bin.end);
                let length = size[0]
                    * if self.hist_logscale {
                        (bin.count as f32).log10() / (max_count as f32).log10()
                    } else {
                        (bin.count as f32) / (max_count as f32)
                    };
                draw_list
                    .add_rect(
                        [x_pos + size[0] - length, y_pos],
                        [x_pos + size[0], y_pos_end],
                        FILL_COLOR,
                    )
                    .filled(true)
                    .build();
            }

            draw_list
                .add_rect(pos, [pos[0] + size[0], pos[1] + size[1]], BORDER_COLOR)
                .build();
        } // TODO show error
    }

    pub(crate) fn show_hist_color(&self, ui: &Ui, pos: [f32; 2], size: [f32; 2]) {
        let vmin = 0.0;
        let vmax = 65535.0;

        const FILL_COLOR_R: u32 = 0x5500_00FF;
        const FILL_COLOR_G: u32 = 0x5500_FF00;
        const FILL_COLOR_B: u32 = 0x55FF_0000;
        const BORDER_COLOR: u32 = 0xFF00_0000;
        let hist = self.image.hist_color();

        let (max_count_r, max_count_g, max_count_b) =
            hist.iter().fold((0, 0, 0), |(max_r, max_g, max_b), bin| {
                (
                    max_r.max(bin[0].count),
                    max_g.max(bin[1].count),
                    max_b.max(bin[2].count),
                )
            });
        let draw_list = ui.get_window_draw_list();
        let max_count = max_count_r.max(max_count_g).max(max_count_b);
        let x_pos = pos[0];
        for i in 0..3 {
            for bin in hist {
                let y_pos = pos[1] + size[1] / (vmax - vmin) * (vmax - bin[i].start);
                let y_pos_end = pos[1] + size[1] / (vmax - vmin) * (vmax - bin[i].end);
                let length = size[0]
                    * if self.hist_logscale {
                        (bin[i].count as f32).log10() / (max_count as f32).log10()
                    } else {
                        (bin[i].count as f32) / (max_count as f32)
                    };
                draw_list
                    .add_rect(
                        [x_pos + size[0] - length, y_pos],
                        [x_pos + size[0], y_pos_end],
                        if i == 0 {
                            FILL_COLOR_R
                        } else if i == 1 {
                            FILL_COLOR_G
                        } else {
                            FILL_COLOR_B
                        },
                    )
                    .filled(true)
                    .build();
            }
        }
        draw_list
            .add_rect(pos, [pos[0] + size[0], pos[1] + size[1]], BORDER_COLOR)
            .build();
        // TODO show error
    }

    pub(crate) fn show_roi_selector(&mut self, ui: &Ui) {
        let any_roi = self.interactions.iter_mut().filter_roi().any(|_| true);
        if any_roi {
            let mut names = vec![format!("None")];
            for name in self.roi_input.roi_names.iter() {
                names.push(name.to_string());
            }
            ui.combo_simple_string(format!("Active ROI"), &mut self.roi_input.selected, &names);
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

    fn make_tooltip_for_color(
        &self,
        (x_p, y_p): (usize, usize),
        x: Option<Measurement>,
        y: Option<Measurement>,
        rval: Measurement,
        gval: Measurement,
        bval: Measurement,
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

        let rval_str = if rval.unit.is_empty() {
            format!("RVAL:    {:.2}", rval.v)
        } else {
            format!("RVAL:    {:.2} {}", rval.v, rval.unit)
        };
        let gval_str = if gval.unit.is_empty() {
            format!("GVAL:    {:.2}", gval.v)
        } else {
            format!("GVAL:    {:.2} {}", gval.v, gval.unit)
        };
        let bval_str = if bval.unit.is_empty() {
            format!("BVAL:    {:.2}", bval.v)
        } else {
            format!("BVAL:    {:.2} {}", bval.v, bval.unit)
        };

        if x.is_some() || y.is_some() {
            format!(
                "{} [at point ({}, {})]\n{}\n{}\n{}",
                xy_str, x_p, y_p, rval_str, gval_str, bval_str
            )
        } else {
            format!("{}\n{}\n{}\n{}", xy_str, rval_str, gval_str, bval_str)
        }
    }
}

#[derive(Copy, Clone)]
pub struct Measurement<'a> {
    pub v: f32,
    pub unit: &'a str,
}
