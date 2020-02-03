use std::borrow::Borrow;
use std::collections::HashMap;
use std::time::Instant;

use glium::backend::Facade;
use imgui::{ComboBox, ImString, Image, MenuItem, MouseButton, MouseCursor, TextureId, Ui};
use imshow::aflak_cake::{Transform, TransformIdx};
use imshow::aflak_primitives::{IOErr, IOValue};
use imshow::node_editor::NodeEditor;
use ndarray::ArrayD;

use super::image;
use super::interactions::{
    Circle, FinedGrainedROI, HorizontalLine, Interaction, InteractionId, InteractionIterMut,
    Interactions, Line, ValueIter, VerticalLine,
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
    /// Mouse position relative to the image (in pixels)
    pub mouse_pos: (f32, f32),
    /// Control whether histogram uses a log scale
    pub hist_logscale: bool,
    lut_min_moving: bool,
    lut_max_moving: bool,
    interactions: Interactions,
    roi_input: RoiInputState,
    circle_input: CircleInputState,
    image: image::Image<I>,
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
            mouse_pos: (f32::NAN, f32::NAN),
            hist_logscale: true,
            lut_min_moving: false,
            lut_max_moving: false,
            interactions: Interactions::new(),
            roi_input: Default::default(),
            circle_input: Default::default(),
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
        texture_id: TextureId,
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

    pub(crate) fn show_bar(&mut self, ui: &Ui, pos: [f32; 2], size: [f32; 2]) -> bool {
        let mut changed = false;

        ui.set_cursor_screen_pos(pos);
        ui.invisible_button(im_str!("image_bar"), size);
        if ui.is_item_hovered() && ui.is_mouse_clicked(MouseButton::Right) {
            ui.open_popup(im_str!("swap-lut"));
        }
        ui.popup(im_str!("swap-lut"), || {
            ui.text("Swap LUT");
            ui.separator();
            for builtin_lut in BuiltinLUT::values() {
                let stack = ui.push_id(*builtin_lut as i32);
                if MenuItem::new(builtin_lut.name()).build(ui) {
                    self.lut.set_gradient(*builtin_lut);
                    changed = true;
                }
                stack.pop(ui);
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
            ui.invisible_button(im_str!("set_min"), [TRIANGLE_WIDTH, TRIANGLE_HEIGHT]);
            if ui.is_item_hovered() {
                ui.set_mouse_cursor(Some(MouseCursor::ResizeNS));
                if ui.is_mouse_clicked(MouseButton::Left) {
                    self.lut_min_moving = true;
                }
            }
            if self.lut_min_moving {
                ui.set_mouse_cursor(Some(MouseCursor::ResizeNS));
                let [_, mouse_y] = ui.io().mouse_pos;
                let min = 1.0 - (mouse_y - pos[1]) / size[1];
                self.lut.set_min(min);
                changed = true;
            }
            if !ui.is_mouse_down(MouseButton::Left) {
                self.lut_min_moving = false;
            }

            // Max triangle
            let max_color = util::to_u32_color(self.lut.color_at(lims.1));
            let y_pos = pos[1] + size[1] * (1.0 - lims.1);
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
                ui.set_mouse_cursor(Some(MouseCursor::ResizeNS));
                if ui.is_mouse_clicked(MouseButton::Left) {
                    self.lut_max_moving = true;
                }
            }
            if self.lut_max_moving {
                ui.set_mouse_cursor(Some(MouseCursor::ResizeNS));
                let [_, mouse_y] = ui.io().mouse_pos;
                let max = 1.0 - (mouse_y - pos[1]) / size[1];
                self.lut.set_max(max);
                changed = true;
            }
            if !ui.is_mouse_down(MouseButton::Left) {
                self.lut_max_moving = false;
            }
        }

        let x_pos = pos[0] + 5.0;
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
        node_editor: &AflakNodeEditor,
    ) -> Result<([[f32; 2]; 2], f32), Error>
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
            [original_size.0 * zoom, original_size.1 * zoom]
        };

        let p = ui.cursor_screen_pos();
        ui.set_cursor_screen_pos([p[0] + y_labels_width, p[1] + IMAGE_TOP_PADDING]);
        let p = ui.cursor_screen_pos();

        Image::new(texture_id, size).build(ui);
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

            if ui.is_mouse_clicked(MouseButton::Right) {
                ui.open_popup(im_str!("add-interaction-handle"))
            }
        }

        let draw_list = ui.get_window_draw_list();
        // Add interaction handlers
        ui.popup(im_str!("add-interaction-handle"), || {
            ui.text("Add interaction handle");
            ui.separator();
            if let Some(menu) = ui.begin_menu(im_str!("Horizontal Line"), true) {
                if MenuItem::new(im_str!("to main editor")).build(ui) {
                    let new =
                        Interaction::HorizontalLine(HorizontalLine::new(self.mouse_pos.1.round()));
                    self.interactions.insert(new);
                }
                for macr in node_editor.macros.macros() {
                    if MenuItem::new(&im_str!("to macro: {}", macr.name())).build(ui) {
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
                menu.end(ui);
            }
            if let Some(menu) = ui.begin_menu(im_str!("Vertical Line"), true) {
                if MenuItem::new(im_str!("to main editor")).build(ui) {
                    let new =
                        Interaction::VerticalLine(VerticalLine::new(self.mouse_pos.0.round()));
                    self.interactions.insert(new);
                }
                for macr in node_editor.macros.macros() {
                    if MenuItem::new(&im_str!("to macro: {}", macr.name())).build(ui) {
                        let new =
                            Interaction::VerticalLine(VerticalLine::new(self.mouse_pos.0.round()));
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
                menu.end(ui);
            }
            if MenuItem::new(im_str!("Region of interest")).build(ui) {
                let new =
                    Interaction::FinedGrainedROI(FinedGrainedROI::new(self.roi_input.gen_id()));
                self.interactions.insert(new);
            }
            if MenuItem::new(im_str!("Line")).build(ui) {
                let new = Interaction::Line(Line::new());
                self.interactions.insert(new);
            }
            if MenuItem::new(im_str!("Circle")).build(ui) {
                let new = Interaction::Circle(Circle::new(self.circle_input.gen_id()));
                self.interactions.insert(new);
            }
            if let Some((id, t_idx)) = *copying {
                ui.separator();
                ui.text("Paste Line Options");
                ui.separator();
                if MenuItem::new(im_str!("Paste Line as Horizontal Line")).build(ui) {
                    let new =
                        Interaction::HorizontalLine(HorizontalLine::new(self.mouse_pos.1.round()));
                    self.interactions.insert(new);
                    store.insert(self.interactions.id(), t_idx);
                    *copying = None;
                }
                if MenuItem::new(im_str!("Paste Line as Vertical Line")).build(ui) {
                    let new =
                        Interaction::VerticalLine(VerticalLine::new(self.mouse_pos.0.round()));
                    self.interactions.insert(new);
                    store.insert(self.interactions.id(), t_idx);
                    *copying = None;
                }
            }
        });

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
                        im_str!("horizontal-line"),
                        [size[0], 2.0 * CLICKABLE_HEIGHT],
                    );
                    if ui.is_item_hovered() {
                        ui.set_mouse_cursor(Some(MouseCursor::ResizeNS));
                        if ui.is_mouse_clicked(MouseButton::Left) {
                            *moving = true;
                        }
                        if ui.is_mouse_clicked(MouseButton::Right) {
                            ui.open_popup(im_str!("edit-horizontal-line"))
                        }
                    }
                    if *moving {
                        *height = util::clamp(self.mouse_pos.1.round(), 0.0, tex_size.1 as f32);
                    }
                    if !ui.is_mouse_down(MouseButton::Left) {
                        *moving = false;
                    }

                    draw_list
                        .add_line([x, y], [x + size[0], y], LINE_COLOR)
                        .build();

                    ui.popup(im_str!("edit-horizontal-line"), || {
                        if MenuItem::new(im_str!("Delete Line")).build(ui) {
                            line_marked_for_deletion = Some(*id);
                        }
                        if MenuItem::new(im_str!("Copy Line")).build(ui) {
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

                    ui.invisible_button(im_str!("vertical-line"), [2.0 * CLICKABLE_WIDTH, size[1]]);
                    if ui.is_item_hovered() {
                        ui.set_mouse_cursor(Some(MouseCursor::ResizeEW));
                        if ui.is_mouse_clicked(MouseButton::Left) {
                            *moving = true;
                        }
                        if ui.is_mouse_clicked(MouseButton::Right) {
                            ui.open_popup(im_str!("edit-vertical-line"))
                        }
                    }
                    if *moving {
                        *x_pos = util::clamp(self.mouse_pos.0.round(), 0.0, tex_size.0 as f32);
                    }
                    if !ui.is_mouse_down(MouseButton::Left) {
                        *moving = false;
                    }

                    draw_list
                        .add_line([x, y], [x, y + size[1]], LINE_COLOR)
                        .build();

                    ui.popup(im_str!("edit-vertical-line"), || {
                        if MenuItem::new(im_str!("Delete Line")).build(ui) {
                            line_marked_for_deletion = Some(*id);
                        }
                        if MenuItem::new(im_str!("Copy Line")).build(ui) {
                            if store.contains_key(id) {
                                let t_idx = *store.get(id).unwrap();
                                *copying = Some((*id, t_idx));
                            } else {
                                println!("copy failued");
                            }
                        }
                    });
                }
                Interaction::FinedGrainedROI(FinedGrainedROI { id, pixels }) => {
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
                        )
                    }

                    if selected && is_image_hovered && ui.is_mouse_clicked(MouseButton::Left) {
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
                    endpoints,
                    endpointsfill,
                    pixels,
                    pre_mousepos,
                    allmoving,
                    edgemoving,
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
                    let y0 = p[1] + size[1] - (endpoints.0).1 as f32 / tex_size.1 as f32 * size[1];
                    let x1 = p[0] + (endpoints.1).0 as f32 / tex_size.0 as f32 * size[0];
                    let y1 = p[1] + size[1] - (endpoints.1).1 as f32 / tex_size.1 as f32 * size[1];
                    let linevec = (x1 - x0, y1 - y0);
                    let linevecsize = (linevec.0 * linevec.0 + linevec.1 * linevec.1).sqrt();
                    let center = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
                    let angle = (linevec.1).atan2(linevec.0);
                    let rotated = (
                        linevec.1 / linevecsize * CLICKABLE_WIDTH,
                        -linevec.0 / linevecsize * CLICKABLE_WIDTH,
                    );
                    let upperleft = (x0 + rotated.0, y0 + rotated.1);
                    let center_to_upperleft = (upperleft.0 - center.0, upperleft.1 - center.1);
                    let mousepos = (
                        p[0] + self.mouse_pos.0 as f32 / tex_size.0 as f32 * size[0],
                        p[1] + size[1] - self.mouse_pos.1 as f32 / tex_size.1 as f32 * size[1],
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
                                    .add_circle([x0, y0], CLICKABLE_WIDTH * 2.0, CIRCLE_COLOR_BEGIN)
                                    .filled(true)
                                    .build();
                            }
                            ui.set_cursor_screen_pos([
                                mousepos.0 - CLICKABLE_WIDTH,
                                mousepos.1 - CLICKABLE_WIDTH,
                            ]);
                            ui.invisible_button(
                                im_str!("line"),
                                [CLICKABLE_WIDTH * 2.0, CLICKABLE_WIDTH * 2.0],
                            );
                            ui.set_mouse_cursor(Some(MouseCursor::ResizeAll));

                            if ui.is_mouse_clicked(MouseButton::Left) && !edgemoving.0 {
                                edgemoving.0 = true;
                            }
                            if ui.is_mouse_clicked(MouseButton::Right) {
                                ui.open_popup(im_str!("edit-line"))
                            }
                        } else if (x1 - mousepos.0) * (x1 - mousepos.0)
                            + (y1 - mousepos.1) * (y1 - mousepos.1)
                            <= CLICKABLE_WIDTH * CLICKABLE_WIDTH
                        {
                            if !edgemoving.1 {
                                const CIRCLE_COLOR_END: u32 = 0x80FF_0000;
                                draw_list
                                    .add_circle([x1, y1], CLICKABLE_WIDTH * 2.0, CIRCLE_COLOR_END)
                                    .filled(true)
                                    .build();
                            }
                            ui.set_cursor_screen_pos([
                                mousepos.0 - CLICKABLE_WIDTH,
                                mousepos.1 - CLICKABLE_WIDTH,
                            ]);
                            ui.invisible_button(
                                im_str!("line"),
                                [CLICKABLE_WIDTH * 2.0, CLICKABLE_WIDTH * 2.0],
                            );
                            ui.set_mouse_cursor(Some(MouseCursor::ResizeAll));

                            if ui.is_mouse_clicked(MouseButton::Left) && !edgemoving.1 {
                                edgemoving.1 = true;
                            }
                            if ui.is_mouse_clicked(MouseButton::Right) {
                                ui.open_popup(im_str!("edit-line"))
                            }
                        } else if rotated_upperleft.0 <= rotated_mousepos.0
                            && rotated_mousepos.0 <= rotated_upperleft.0 + linevecsize
                            && rotated_upperleft.1 <= rotated_mousepos.1
                            && rotated_mousepos.1 <= rotated_upperleft.1 + CLICKABLE_WIDTH * 2.0
                        {
                            ui.set_cursor_screen_pos([
                                mousepos.0 - CLICKABLE_WIDTH,
                                mousepos.1 - CLICKABLE_WIDTH,
                            ]);
                            ui.invisible_button(
                                im_str!("line"),
                                [CLICKABLE_WIDTH * 2.0, CLICKABLE_WIDTH * 2.0],
                            );
                            ui.set_mouse_cursor(Some(MouseCursor::ResizeAll));

                            if ui.is_mouse_clicked(MouseButton::Left) && !*allmoving {
                                *allmoving = true;
                                *pre_mousepos = self.mouse_pos;
                            }
                            if ui.is_mouse_clicked(MouseButton::Right) {
                                ui.open_popup(im_str!("edit-line"))
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
                                .add_circle([x0, y0], CLICKABLE_WIDTH * 2.0, CIRCLE_COLOR_BEGIN)
                                .filled(true)
                                .build();
                            endpoints.0 = self.mouse_pos;
                            pixels.clear();
                            get_pixels_of_line(pixels, *endpoints, tex_size);
                        } else if edgemoving.1 {
                            const CIRCLE_COLOR_END: u32 = 0x80FF_0000;
                            draw_list
                                .add_circle([x1, y1], CLICKABLE_WIDTH * 2.0, CIRCLE_COLOR_END)
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

                        ui.popup(im_str!("edit-line"), || {
                            if MenuItem::new(im_str!("Delete Line")).build(ui) {
                                line_marked_for_deletion = Some(*id);
                            }
                        });
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
                        let wx = ((endpoints.1).0 as isize - (endpoints.0).0 as isize).abs();
                        let wy = ((endpoints.1).1 as isize - (endpoints.0).1 as isize).abs();
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
                    if selected && is_image_hovered && ui.is_mouse_clicked(MouseButton::Left) {
                        let pixel = (self.mouse_pos.0 as usize, self.mouse_pos.1 as usize);
                        if parametersfill.0 == false {
                            parametersfill.0 = true;
                            *center = pixel;
                        } else if parametersfill.1 == false {
                            parametersfill.1 = true;
                            let x0 = p[0] + center.0 as f32 / tex_size.0 as f32 * size[0];
                            let y0 = p[1] + size[1] - center.1 as f32 / tex_size.1 as f32 * size[1];
                            let x1 = p[0] + self.mouse_pos.0 as f32 / tex_size.0 as f32 * size[0];
                            let y1 = p[1] + size[1]
                                - self.mouse_pos.1 as f32 / tex_size.1 as f32 * size[1];
                            let rad = ((x0 - x1) * (x0 - x1) + (y0 - y1) * (y0 - y1)).sqrt();
                            *radius = rad;
                        }
                    }

                    if parametersfill.0 && parametersfill.1 {
                        let x0 = p[0] + center.0 as f32 / tex_size.0 as f32 * size[0];
                        let y0 = p[1] + size[1] - center.1 as f32 / tex_size.1 as f32 * size[1];
                        draw_list
                            .add_circle([x0, y0], *radius as f32, LINE_COLOR)
                            .num_segments(50)
                            .build();
                    } else if parametersfill.0 {
                        let x0 = p[0] + center.0 as f32 / tex_size.0 as f32 * size[0];
                        let y0 = p[1] + size[1] - center.1 as f32 / tex_size.1 as f32 * size[1];
                        let x1 = p[0] + self.mouse_pos.0 as f32 / tex_size.0 as f32 * size[0];
                        let y1 =
                            p[1] + size[1] - self.mouse_pos.1 as f32 / tex_size.1 as f32 * size[1];
                        let rad = ((x0 - x1) * (x0 - x1) + (y0 - y1) * (y0 - y1)).sqrt();
                        draw_list
                            .add_circle([x0, y0], rad, LINE_COLOR)
                            .num_segments(50)
                            .build();
                    }
                }
            }
            stack.pop(ui);
        }

        if let Some(line_id) = line_marked_for_deletion {
            self.interactions.remove(line_id);
        }

        ticks.draw(&draw_list, p, size);

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

    pub(crate) fn show_roi_selector(&mut self, ui: &Ui) {
        let any_roi = self.interactions.iter_mut().filter_roi().any(|_| true);
        if any_roi {
            let mut names = vec![im_str!("None")];
            for name in self.roi_input.roi_names.iter() {
                names.push(&name);
            }
            ComboBox::new(im_str!("Active ROI")).build_simple_string(
                ui,
                &mut self.roi_input.selected,
                &names,
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
