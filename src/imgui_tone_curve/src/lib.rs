extern crate imgui;
extern crate serde;
extern crate serde_derive;

use imgui::*;
use serde::{Deserialize, Serialize};
use std::io;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToneCurveState {
    arr: Vec<f32>,
    cp: Vec<[f32; 2]>,
    adding: Option<[f32; 2]>,
    pushed: bool,
    moving: Option<usize>,
    deleting: Option<usize>,
    x_clicking: usize,
    is_dragging: bool,
}

impl ToneCurveState {
    pub fn default() -> Self {
        ToneCurveState {
            arr: {
                let mut vec = vec![];
                for i in 0..256 {
                    vec.push(i as f32);
                }
                vec
            },
            cp: vec![[0.0, 0.0], [1.0, 1.0]],
            adding: None,
            pushed: false,
            moving: None,
            deleting: None,
            x_clicking: 0,
            is_dragging: false,
        }
    }

    pub fn control_points(&self) -> &Vec<[f32; 2]> {
        &self.cp
    }

    pub fn array(&self) -> &Vec<f32> {
        &self.arr
    }
}

impl PartialEq for ToneCurveState {
    fn eq(&self, val: &Self) -> bool {
        let cp1 = self.control_points();
        let cp2 = val.control_points();
        cp1 == cp2 && self.deleting == val.deleting
    }
}

pub trait UiToneCurve {
    fn create_curve(cp: &Vec<[f32; 2]>) -> Vec<f32>;
    fn tone_curve(
        &self,
        state: &mut ToneCurveState,
        draw_list: &DrawListMut,
    ) -> io::Result<Option<ToneCurveState>>;
}

impl<'ui> UiToneCurve for Ui<'ui> {
    fn create_curve(cp: &Vec<[f32; 2]>) -> Vec<f32> {
        let mut ret = Vec::new();
        match cp.len() {
            0 | 1 => {
                vec![]
            }
            2 => {
                let slope = (cp[1][1] - cp[0][1]) / (cp[1][0] - cp[0][0]);
                let ys = cp[0][1];
                for i in 0..256 {
                    let t = i as f32 / 256.0;
                    ret.push((t * slope + ys) * 256.0);
                }
                ret
            }
            3 => {
                let mut cp = cp.clone();
                cp.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap());
                let (x1, x2, x3, y1, y2, y3) =
                    (cp[0][0], cp[1][0], cp[2][0], cp[0][1], cp[1][1], cp[2][1]);
                let a = ((y1 - y2) * (x1 - x3) - (y1 - y3) * (x1 - x2))
                    / ((x1 - x2) * (x1 - x3) * (x2 - x3));
                let b = (y1 - y2) / (x1 - x2) - a * (x1 + x2);
                let c = y1 - a * x1 * x1 - b * x1;
                for i in 0..256 {
                    let t = i as f32 / 256.0;
                    ret.push((a * t * t + b * t + c) * 256.0);
                }
                ret
            }
            _ => {
                let mut cp = cp.clone();
                cp.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap());
                let mut counter = 0;
                for _ in 0..cp.len() - 1 {
                    if counter == 0 {
                        let p0 = cp[0];
                        let p1 = cp[1];
                        let p2 = cp[2];
                        let size = (p1[0] * 256.0).ceil() as usize;
                        let b = [p0[0] - 2.0 * p1[0] + p2[0], p0[1] - 2.0 * p1[1] + p2[1]];
                        let c = [
                            -3.0 * p0[0] + 4.0 * p1[0] - p2[0],
                            -3.0 * p0[1] + 4.0 * p1[1] - p2[1],
                        ];
                        let d = [2.0 * p0[0], 2.0 * p0[1]];
                        for i in 0..size {
                            let t = i as f32 / (size - 1) as f32;
                            ret.push(((b[1] * t * t) + (c[1] * t) + d[1]) * 128.0);
                        }
                    } else if counter == cp.len() - 2 {
                        let p0 = cp[counter - 1];
                        let p1 = cp[counter];
                        let p2 = cp[counter + 1];
                        let size = ((p2[0] * 256.0).floor() - (p1[0] * 256.0).ceil()) as usize;
                        let b = [p0[0] - 2.0 * p1[0] + p2[0], p0[1] - 2.0 * p1[1] + p2[1]];
                        let c = [-p0[0] + p2[0], -p0[1] + p2[1]];
                        let d = [2.0 * p1[0], 2.0 * p1[1]];
                        for i in 0..size {
                            let t = i as f32 / (size - 1) as f32;
                            ret.push(((b[1] * t * t) + (c[1] * t) + d[1]) * 128.0);
                        }
                    } else {
                        let p0 = cp[counter - 1];
                        let p1 = cp[counter];
                        let p2 = cp[counter + 1];
                        let p3 = cp[counter + 2];
                        let size = ((p2[0] * 256.0).floor() - (p1[0] * 256.0).ceil()) as usize + 1;
                        let a = [
                            -p0[0] + 3.0 * p1[0] - 3.0 * p2[0] + p3[0],
                            -p0[1] + 3.0 * p1[1] - 3.0 * p2[1] + p3[1],
                        ];
                        let b = [
                            2.0 * p0[0] - 5.0 * p1[0] + 4.0 * p2[0] - p3[0],
                            2.0 * p0[1] - 5.0 * p1[1] + 4.0 * p2[1] - p3[1],
                        ];
                        let c = [-p0[0] + p2[0], -p0[1] + p2[1]];
                        let d = [2.0 * p1[0], 2.0 * p1[1]];
                        for i in 0..size {
                            let t = i as f32 / (size - 1) as f32;
                            ret.push(
                                ((a[1] * t * t * t) + (b[1] * t * t) + (c[1] * t) + d[1]) * 128.0,
                            );
                        }
                    }
                    counter += 1;
                }
                ret
            }
        }
    }

    fn tone_curve(
        &self,
        state: &mut ToneCurveState,
        draw_list: &DrawListMut,
    ) -> io::Result<Option<ToneCurveState>> {
        let p = self.cursor_screen_pos();
        let mouse_pos = self.io().mouse_pos;
        let [mouse_x, mouse_y] = [mouse_pos[0] - p[0] - 5.0, mouse_pos[1] - p[1] - 5.0];
        state.arr = Self::create_curve(&state.cp);
        self.invisible_button(format!("tone_curve"), [410.0, 410.0]);
        self.set_cursor_screen_pos(p);
        PlotLines::new(self, format!(""), &state.arr)
            .graph_size([410.0, 410.0])
            .scale_min(0.0)
            .scale_max(256.0)
            .build();
        self.set_cursor_screen_pos(p);
        self.invisible_button(format!("tone_curve"), [410.0, 410.0]);
        if let Some(adding) = state.adding {
            let x = adding[0] * 400.0 + 5.0 + p[0];
            let y = (1.0 - adding[1]) * 400.0 + 5.0 + p[1];
            draw_list.add_circle([x, y], 5.0, 0xFF00_FFFF).build();
        }
        let mut counter = 0;
        for i in &state.cp {
            let x = i[0] * 400.0 + 5.0 + p[0];
            let y = (1.0 - i[1]) * 400.0 + 5.0 + p[1];
            if (x - mouse_pos[0]) * (x - mouse_pos[0]) + (y - mouse_pos[1]) * (y - mouse_pos[1])
                < 25.0
            {
                draw_list.add_circle([x, y], 5.0, 0xFF00_00FF).build();
                if self.is_mouse_clicked(MouseButton::Left) && state.adding == None {
                    state.moving = Some(counter);
                }
                if self.is_mouse_clicked(MouseButton::Right) {
                    self.open_popup(format!("delete-control-point"));
                    state.deleting = Some(counter);
                }
            } else {
                draw_list.add_circle([x, y], 5.0, 0xFFFF_FFFF).build();
            }
            counter += 1;
        }
        self.popup(format!("delete-control-point"), || {
            if MenuItem::new(format!("Delete Control Point")).build(self) {
                if let Some(key) = state.deleting {
                    state.cp.remove(key);
                    state.deleting = None;
                }
            }
        });
        if self.is_item_hovered() {
            if self.is_mouse_clicked(MouseButton::Left) && state.moving == None {
                if !state.is_dragging {
                    //let x = 256.0 * mouse_x / 400.0;
                    //state.x_clicking = x as usize;
                    state.is_dragging = true;
                }
            }
            if state.is_dragging {
                if state.pushed == false {
                    state.pushed = true;
                    state.cp.push([mouse_x / 400.0, (400.0 - mouse_y) / 400.0]);
                }
                state.adding = Some([mouse_x / 400.0, (400.0 - mouse_y) / 400.0]);
                let lastidx = state.cp.len() - 1;
                state.cp[lastidx] = state.adding.unwrap();
                if state.x_clicking > 255 {
                    state.x_clicking = 255;
                }
                //state.arr[state.x_clicking] = 256.0 * (400.0 - mouse_y) / 400.0;
                /*if state.x_clicking == 255 {
                    let lastidx = state.cp.len() - 1;
                    state.cp[lastidx] = [1.0, (400.0 - mouse_y) / 400.0];
                } else if state.x_clicking == 0 {
                    state.cp[0] = [0.0, (400.0 - mouse_y) / 400.0];
                } else {

                }*/
                if !self.is_mouse_down(MouseButton::Left) {
                    state.cp.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap());
                    state.adding = None;
                    state.is_dragging = false;
                    state.pushed = false;
                }
                /*if !self.is_mouse_down(MouseButton::Left) {
                        println!("{}", state.x_clicking);
                        state.cp.push([mouse_x / 400.0, (400.0 - mouse_y) / 400.0]);
                        state.cp.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap());
                    state.is_dragging = false;
                    state.x_clicking = 0;
                }*/
            }
            if let Some(key) = state.moving {
                state.cp[key] = [mouse_x / 400.0, (400.0 - mouse_y) / 400.0];
                if !self.is_mouse_down(MouseButton::Left) {
                    state.moving = None;
                }
            }
        }
        let state = state.clone();
        Ok(Some(state))
    }
}
