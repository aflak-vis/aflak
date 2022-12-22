use imgui::Ui;
use std::collections::HashMap;

use super::interactions::{
    ColorLut, Interaction, InteractionId, InteractionIterMut, Interactions, ValueIter,
};
use super::node_editor::NodeEditor;
use super::primitives::{IOErr, IOValue};
use super::Error;

use crate::plot_colormap::cake::{OutputId, TransformIdx};
use imgui::{ColorEdit, MenuItem, MouseButton, MouseCursor};

type EditableValues = HashMap<InteractionId, TransformIdx>;
type AflakNodeEditor = NodeEditor<IOValue, IOErr>;

/// Current state of a plot UI.
#[derive(Debug)]
pub struct State {
    offset: [f32; 2],
    zoom: [f32; 2],
    //mouse_pos: [f32; 2],
    interactions: Interactions,
    any_moving: Option<i32>,
    any_changing: Option<i32>,
    changing_lut: Option<(f32, [u8; 3])>,
    color_edit: Vec<[f32; 3]>,
    pub colormode: [bool; 2],
    color_adding: Option<(usize, (f32, [u8; 3]))>,
    color_deleting: Option<usize>,
}

impl Default for State {
    fn default() -> Self {
        State {
            offset: [0.0, 0.0],
            zoom: [1.0, 1.0],
            //mouse_pos: [f32::NAN, f32::NAN],
            interactions: Interactions::new(),
            any_moving: None,
            any_changing: None,
            changing_lut: None,
            color_edit: vec![],
            colormode: [true, false],
            color_adding: None,
            color_deleting: None,
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

    pub(crate) fn plot_colormap(
        &mut self,
        ui: &Ui,
        colormap: &(usize, Vec<(f32, [u8; 3])>),
        _vtype: &str,
        //vunit: &str,
        //axis: Option<&AxisTransform<F>>,
        pos: [f32; 2],
        size: [f32; 2],
        _copying: &mut Option<(InteractionId, TransformIdx)>,
        store: &mut EditableValues,
        attaching: &mut Option<(OutputId, TransformIdx, usize)>,
        outputid: OutputId,
        _node_editor: &AflakNodeEditor,
    ) -> Result<(), Error> {
        /*let min = lims::get_vmin(image)?;
        let max = lims::get_vmax(image)?;*/
        if let Some((o, t_idx, kind)) = *attaching {
            if o == outputid && kind == 3 {
                let mut already_insert = false;
                for d in store.iter() {
                    if *d.1 == t_idx {
                        already_insert = true;
                        break;
                    }
                }
                if !already_insert {
                    let new = Interaction::ColorLut(ColorLut::new(colormap.0, colormap.1.to_vec()));
                    self.interactions.insert(new);
                    store.insert(self.interactions.id(), t_idx);
                } else {
                    eprintln!("{:?} is already bound", t_idx)
                }
                *attaching = None;
            }
        }

        let xvlims = (0.0, 1.0);
        let yvlims = (0.0, 255.0);
        let xlims = (
            xvlims.0 * self.zoom[0] + self.offset[0],
            xvlims.1 * self.zoom[0] + self.offset[0],
        );
        let ylims = (
            yvlims.0 * self.zoom[1] + self.offset[1],
            yvlims.1 * self.zoom[1] + self.offset[1],
        );

        // Start drawing the figure
        ui.set_cursor_screen_pos([pos[0] + 30.0 /*y_labels_width*/, pos[1]]);
        let p = ui.cursor_screen_pos();

        ui.invisible_button(format!("plot"), size);
        ui.set_item_allow_overlap();

        let draw_list = ui.get_window_draw_list();

        const BG_COLOR: u32 = 0xA033_3333;
        const LINE_COLOR: u32 = 0xFFFF_FFFF;
        const LINE_COLORS: [u32; 3] = [0xFF00_00FF, 0xFF00_FF00, 0xFFFF_0000];
        const OFFSET_X: f32 = 60.0;
        const OFFSET_Y: f32 = 60.0;

        let bottom_right_corner = [p[0] + size[0] - OFFSET_X, p[1] + size[1] - OFFSET_Y];

        draw_list.with_clip_rect_intersect(p, bottom_right_corner, || {
            draw_list
                .add_rect(p, bottom_right_corner, BG_COLOR)
                .filled(true)
                .build();

            let first = colormap.1.iter();
            let second = colormap.1.iter().skip(1);

            for (y1, y2) in first.zip(second) {
                if self.colormode[0] == true {
                    for i in 0..3 {
                        let x1 = y1.0 as f32;
                        let x2 = y2.0 as f32;
                        let actual_value1 = match colormap.0 {
                            0 => y1.1,
                            1 => self.hsv2rgb(y1.1),
                            _ => unimplemented!(),
                        };
                        let actual_value2 = match colormap.0 {
                            0 => y2.1,
                            1 => self.hsv2rgb(y2.1),
                            _ => unimplemented!(),
                        };
                        let y1 = actual_value1[i] as f32;
                        let y2 = actual_value2[i] as f32;
                        let p0 = [
                            p[0] + (x1 - xlims.0) / (xlims.1 - xlims.0) * (size[0] - OFFSET_X),
                            p[1] + size[1]
                                - OFFSET_Y
                                - (y1 - ylims.0) / (ylims.1 - ylims.0) * (size[1] - OFFSET_Y),
                        ];
                        let p1 = [
                            p[0] + (x2 - xlims.0) / (xlims.1 - xlims.0) * (size[0] - OFFSET_X),
                            p[1] + size[1]
                                - OFFSET_Y
                                - (y2 - ylims.0) / (ylims.1 - ylims.0) * (size[1] - OFFSET_Y),
                        ];
                        draw_list.add_line(p0, p1, LINE_COLORS[i]).build();
                    }
                } else if self.colormode[1] == true {
                    for i in 0..3 {
                        let x1 = y1.0 as f32;
                        let x2 = y2.0 as f32;
                        let actual_value1 = match colormap.0 {
                            0 => self.rgb2hsv(y1.1),
                            1 => y1.1,
                            _ => unimplemented!(),
                        };
                        let actual_value2 = match colormap.0 {
                            0 => self.rgb2hsv(y2.1),
                            1 => y2.1,
                            _ => unimplemented!(),
                        };
                        let y1 = actual_value1[i] as f32;
                        let y2 = actual_value2[i] as f32;
                        let p0 = [
                            p[0] + (x1 - xlims.0) / (xlims.1 - xlims.0) * (size[0] - OFFSET_X),
                            p[1] + size[1]
                                - OFFSET_Y
                                - (y1 - ylims.0) / (ylims.1 - ylims.0) * (size[1] - OFFSET_Y),
                        ];
                        let p1 = [
                            p[0] + (x2 - xlims.0) / (xlims.1 - xlims.0) * (size[0] - OFFSET_X),
                            p[1] + size[1]
                                - OFFSET_Y
                                - (y2 - ylims.0) / (ylims.1 - ylims.0) * (size[1] - OFFSET_Y),
                        ];
                        draw_list.add_line(p0, p1, LINE_COLORS[i]).build();
                    }
                }
            }
        });

        ui.set_cursor_screen_pos([pos[0] + 30.0, pos[1] + size[1] - 30.0]);
        let p = ui.cursor_screen_pos();
        draw_list
            .add_line(
                [p[0], p[1]],
                [p[0] + (size[0] - OFFSET_X), p[1]],
                LINE_COLOR,
            )
            .build();

        let mouse_pos = ui.io().mouse_pos;
        ui.set_cursor_screen_pos([p[0], p[1] - 9.0]);
        ui.invisible_button(format!("tf_control_bar"), [size[0] - OFFSET_X, 13.0]);
        if ui.is_item_hovered() && ui.is_mouse_clicked(MouseButton::Right) {
            let mouse_x = mouse_pos[0];
            let v = (mouse_x - p[0]) / (size[0] - OFFSET_X);
            let prev = colormap.1.iter();
            let next = colormap.1.iter().skip(1);
            let mut insert_pos = 1;
            let mut r = 0;
            let mut g = 0;
            let mut b = 0;
            for (p, n) in prev.zip(next) {
                if p.0 < v && v < n.0 {
                    let coef = (v - p.0) / (n.0 - p.0);
                    let r1 = p.1[0] as f32;
                    let r2 = n.1[0] as f32;
                    let g1 = p.1[1] as f32;
                    let g2 = n.1[1] as f32;
                    let b1 = p.1[2] as f32;
                    let b2 = n.1[2] as f32;
                    r = (r1 + (r2 - r1) * coef) as u8;
                    g = (g1 + (g2 - g1) * coef) as u8;
                    b = (b1 + (b2 - b1) * coef) as u8;
                    break;
                }
                insert_pos += 1;
            }
            self.color_adding = Some((insert_pos, (v, [r, g, b])));
            ui.open_popup("tf_control");
        }

        let mut counter = 0;
        self.color_edit.clear();
        for (_, color) in &colormap.1 {
            if colormap.0 == 0 {
                self.color_edit.push([
                    color[0] as f32 / 255.0,
                    color[1] as f32 / 255.0,
                    color[2] as f32 / 255.0,
                ]);
            } else if colormap.0 == 1 {
                let color = self.hsv2rgb(*color);
                self.color_edit.push([
                    color[0] as f32 / 255.0,
                    color[1] as f32 / 255.0,
                    color[2] as f32 / 255.0,
                ]);
            }
        }
        ui.popup("tf_control", || {
            if MenuItem::new("Add new").build(ui) {
                for (id, interaction) in self.interactions.iter_mut() {
                    let stack = ui.push_id(id.id());
                    match interaction {
                        Interaction::ColorLut(ColorLut { lut, .. }) => {
                            if let Some((pos, c)) = self.color_adding {
                                lut.insert(pos, c);
                                self.color_adding = None;
                            }
                        }
                        _ => {}
                    }
                    stack.pop();
                }
            }
        });
        for (v, _) in &colormap.1 {
            let x = p[0] + (size[0] - OFFSET_X) * (*v);
            let y = p[1];
            ui.set_cursor_screen_pos([x - 9.0, y - 9.0]);
            if ColorEdit::new(
                format!("##{}", counter),
                self.color_edit.get_mut(counter).unwrap(),
            )
            .inputs(false)
            .build(ui)
            {
                if let Some(edited_color) = self.color_edit.get(counter) {
                    let edited_color = &[
                        (edited_color[0] * 255.0) as u8,
                        (edited_color[1] * 255.0) as u8,
                        (edited_color[2] * 255.0) as u8,
                    ];
                    self.any_changing = Some(counter as i32);
                    self.changing_lut = Some((*v, *edited_color));
                }
            }
            //draw_list.add_circle([x, y], 10.0, c).filled(true).build();
            if ui.is_item_hovered() && 0 < counter && counter < colormap.1.len() - 1 {
                ui.set_mouse_cursor(Some(MouseCursor::ResizeEW));
                if ui.is_mouse_clicked(MouseButton::Left) {
                    self.any_moving = Some(counter as i32);
                } else if ui.is_mouse_clicked(MouseButton::Right) {
                    self.color_deleting = Some(counter);
                    ui.open_popup("delete-color");
                }
            }
            counter += 1;
        }
        ui.set_cursor_screen_pos(p);
        if !ui.is_mouse_down(MouseButton::Left) {
            self.any_moving = None;
            self.any_changing = None;
        }
        if let Some(c) = self.any_moving {
            let mouse_x = mouse_pos[0];
            let mut v = (mouse_x - p[0]) / (size[0] - OFFSET_X);
            let prev_v = colormap.1.get(c as usize - 1).unwrap().0;
            let next_v = colormap.1.get(c as usize + 1).unwrap().0;
            if v < prev_v {
                v = prev_v;
            } else if v > next_v {
                v = next_v;
            }
            let color = colormap.1.get(c as usize).unwrap().1;
            self.changing_lut = Some((v, color));
        }
        for (id, interaction) in self.interactions.iter_mut() {
            let stack = ui.push_id(id.id());
            match interaction {
                Interaction::ColorLut(ColorLut { lut, .. }) => {
                    if let Some(c) = self.any_moving {
                        if let Some(l) = self.changing_lut {
                            lut[c as usize] = l;
                        }
                    } else if let Some(c) = self.any_changing {
                        if let Some(l) = self.changing_lut {
                            lut[c as usize] = l;
                        }
                    }
                }
                _ => {}
            }
            stack.pop();
        }
        ui.popup("delete-color", || {
            if MenuItem::new("Delete Color").build(ui) {
                for (id, interaction) in self.interactions.iter_mut() {
                    let stack = ui.push_id(id.id());
                    match interaction {
                        Interaction::ColorLut(ColorLut { lut, .. }) => {
                            if let Some(c) = self.color_deleting {
                                lut.remove(c);
                                self.color_deleting = None;
                            }
                        }
                        _ => {}
                    }
                    stack.pop();
                }
            }
        });

        Ok(())
    }

    fn rgb2hsv(&self, rgb: [u8; 3]) -> [u8; 3] {
        let max = if rgb[0] >= rgb[1] && rgb[0] >= rgb[2] {
            0
        } else if rgb[1] >= rgb[2] && rgb[1] >= rgb[0] {
            1
        } else {
            2
        };
        let min = if rgb[0] <= rgb[1] && rgb[0] <= rgb[2] {
            0
        } else if rgb[1] <= rgb[2] && rgb[1] <= rgb[0] {
            1
        } else {
            2
        };
        let hue = if max == 0 {
            60.0 * ((rgb[1] - rgb[2]) as f32 / (rgb[max] - rgb[min]) as f32)
        } else if max == 1 {
            60.0 * ((rgb[2] - rgb[0]) as f32 / (rgb[max] - rgb[min]) as f32) + 120.0
        } else {
            60.0 * ((rgb[0] - rgb[1]) as f32 / (rgb[max] - rgb[min]) as f32) + 240.0
        };
        let hue = (hue / 360.0 * 255.0) as u8;
        let sat = ((rgb[max] - rgb[min]) as f32 / rgb[max] as f32 * 255.0) as u8;
        let v = rgb[max];
        [hue, sat, v]
    }

    fn hsv2rgb(&self, hsv: [u8; 3]) -> [u8; 3] {
        let h = hsv[0] as f32 / 255.0 * 360.0;
        let (s, v) = (hsv[1] as f32, hsv[2] as f32);
        let max = v as f32;
        let min = max - ((s / 255.0) * max);
        if 0.0 <= h && h < 60.0 {
            let g = (h / 60.0) * (max - min) + min;
            [max as u8, g as u8, min as u8]
        } else if 60.0 <= h && h < 120.0 {
            let r = ((120.0 - h) / 60.0) * (max - min) + min;
            [r as u8, max as u8, min as u8]
        } else if 120.0 <= h && h < 180.0 {
            let b = ((h - 120.0) / 60.0) * (max - min) + min;
            [min as u8, max as u8, b as u8]
        } else if 180.0 <= h && h < 240.0 {
            let g = ((240.0 - h) / 60.0) * (max - min) + min;
            [min as u8, g as u8, max as u8]
        } else if 240.0 <= h && h < 300.0 {
            let r = ((h - 240.0) / 60.0) * (max - min) + min;
            [r as u8, min as u8, max as u8]
        } else if 300.0 <= h && h < 360.0 {
            let b = ((360.0 - h) / 60.0) * (max - min) + min;
            [max as u8, min as u8, b as u8]
        } else {
            [0, 0, 0]
        }
    }
}

#[derive(Copy, Clone)]
pub struct Measurement<'a> {
    pub v: f32,
    pub unit: &'a str,
}
