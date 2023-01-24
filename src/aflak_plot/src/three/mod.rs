//! Draw 3D representations.
pub extern crate aflak_primitives as primitives;
use glium::{
    backend::Facade,
    implement_vertex,
    texture::{ClientFormat, RawImage2d},
    uniform,
    uniforms::{MagnifySamplerFilter, SamplerBehavior},
    Surface, Texture2d,
};
use imgui::{
    ChildWindow, ColorEdit, Condition, ImString, Image, MenuItem, MouseButton, MouseCursor, Slider,
    TextureId, Ui, Window,
};
use imgui_glium_renderer::Texture;
use ndarray::{ArrayBase, Data, Ix3, IxDyn};
use std::borrow::Cow;
use std::rc::Rc;
mod lut;
mod state;

pub use self::state::State;
use super::imshow::Textures;
use super::lims;
use lut::BuiltinLUT;
use primitives::Topology;

/// TODO
pub trait UiImage3d {
    fn image3d<S, F>(
        &self,
        image: &ArrayBase<S, IxDyn>,
        topology: &Option<Topology>,
        texture_id: TextureId,
        textures: &mut Textures,
        ctx: &F,
        state: &mut State,
    ) where
        S: Data<Elem = f32>,
        F: Facade;
}

impl<'ui> UiImage3d for Ui<'ui> {
    // TODO
    fn image3d<S, F>(
        &self,
        image: &ArrayBase<S, IxDyn>,
        topology: &Option<Topology>,
        texture_id: TextureId,
        textures: &mut Textures,
        ctx: &F,
        state: &mut State,
    ) where
        S: Data<Elem = f32>,
        F: Facade,
    {
        let p = self.cursor_screen_pos();
        let window_pos = self.window_pos();
        let window_size = self.window_size();
        let size = [
            window_size[0] - 65.0,
            window_size[1] - (p[1] - window_pos[1]) - 10.0,
        ];

        // 3D image...
        let raw = make_raw_image(image, topology, state);
        let gl_texture = Texture2d::new(ctx, raw).expect("Error!");
        textures.replace(
            texture_id,
            Texture {
                texture: Rc::new(gl_texture),
                sampler: SamplerBehavior {
                    magnify_filter: MagnifySamplerFilter::Nearest,
                    ..Default::default()
                },
            },
        );
        let childwindow_size = window_size;
        ChildWindow::new("scrolling_region")
            .size(childwindow_size)
            .border(false)
            .scroll_bar(false)
            .movable(false)
            .scrollable(false)
            .horizontal_scrollbar(false)
            .build(self, || {
                if state.show_tf_parameters {
                    Window::new(&ImString::new(format!("Brightness Settings")))
                        .size([300.0, 100.0], Condition::Appearing)
                        .resizable(false)
                        .build(self, || {
                            Slider::new(format!("Brightness"), 0.0, 200.0)
                                .build(self, &mut state.topology_brightness);
                        });
                }
                if state.show_single_contour {
                    Window::new(&ImString::new(format!("Single Contour")))
                        .size([300.0, 50.0], Condition::Appearing)
                        .resizable(false)
                        .build(self, || {
                            Slider::new(format!("Value"), 0.01, 0.99)
                                .build(self, &mut state.single_contour);
                        });
                    let gradient = state.lut.gradient();
                    let readmode = state.lut.read_mode();
                    let mut gradient_alpha = vec![(0.0 as f32, 0 as u8)];
                    gradient_alpha.push((state.single_contour - 0.005, 0));
                    gradient_alpha.push((state.single_contour, 255));
                    gradient_alpha.push((state.single_contour + 0.005, 0));
                    gradient_alpha.push((1.0, 0));
                    state.lut.set_gradient((gradient, gradient_alpha, readmode));
                } else if state.critical_isosurface {
                    let gradient = state.lut.gradient();
                    let readmode = state.lut.read_mode();
                    let mut gradient_alpha = vec![(0.0 as f32, 0 as u8)];
                    if let Some(topology) = topology {
                        let mut cp_vals = vec![];
                        let vmin = lims::get_vmin(&image).unwrap();
                        let vmax = lims::get_vmax(&image).unwrap();
                        for c in &topology.critical_points {
                            if c.point_type == 0 || c.point_type == 3 {
                                let coord =
                                    [c.coord.2 as usize, c.coord.0 as usize, c.coord.1 as usize];
                                if let Some(v) = image.get(coord) {
                                    let v = (*v - vmin) / (vmax - vmin);
                                    if !v.is_nan() {
                                        cp_vals.push(v);
                                    }
                                }
                            }
                        }
                        cp_vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        cp_vals.dedup_by(|a, b| (*a - *b).abs() < state.topology_interval * 1.0e-2);
                        let cp_vals_len = cp_vals.len() - 1;
                        for (k, v) in cp_vals.iter().enumerate() {
                            let alpha = (255.0 * k as f32 / cp_vals_len as f32) as u8;
                            gradient_alpha.push((*v - 0.005, 0));
                            gradient_alpha.push((*v, alpha));
                            gradient_alpha.push((*v + 0.005, 0));
                        }
                    }

                    gradient_alpha.push((1.0, 0));
                    state.lut.set_gradient((gradient, gradient_alpha, readmode));
                } else if state.representative_isosurface {
                    let gradient = state.lut.gradient();
                    let readmode = state.lut.read_mode();
                    let mut gradient_alpha = vec![(0.0 as f32, 0 as u8)];
                    if let Some(topology) = topology {
                        let mut cp_vals = vec![0.0];
                        let mut rep_vals = vec![];
                        let vmin = lims::get_vmin(&image).unwrap();
                        let vmax = lims::get_vmax(&image).unwrap();
                        for c in &topology.critical_points {
                            if c.point_type == 0 || c.point_type == 3 {
                                let coord =
                                    [c.coord.2 as usize, c.coord.0 as usize, c.coord.1 as usize];

                                if let Some(v) = image.get(coord) {
                                    let v = (*v - vmin) / (vmax - vmin);
                                    if !v.is_nan() {
                                        cp_vals.push(v);
                                    }
                                }
                            }
                        }
                        cp_vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        cp_vals.dedup_by(|a, b| (*a - *b).abs() < state.topology_interval * 1.0e-2);
                        cp_vals.push(1.0);
                        let first = cp_vals.iter();
                        let second = cp_vals.iter().skip(1);
                        for (v1, v2) in first.zip(second) {
                            rep_vals.push((v1 + v2) / 2.0);
                        }
                        let rep_len = rep_vals.len() - 1;
                        for (k, v) in rep_vals.iter().enumerate() {
                            let alpha = (255.0 * k as f32 / rep_len as f32) as u8;
                            gradient_alpha.push((*v - 0.005, 0));
                            gradient_alpha.push((*v, alpha));
                            gradient_alpha.push((*v + 0.005, 0));
                        }
                    }

                    gradient_alpha.push((1.0, 0));
                    state.lut.set_gradient((gradient, gradient_alpha, readmode));
                } else if state.single_contour_clicked {
                    let gradient = state.lut.gradient();
                    let readmode = state.lut.read_mode();
                    let gradient_alpha = vec![(0.0 as f32, 0 as u8), (1.0 as f32, 255 as u8)];
                    state.lut.set_gradient((gradient, gradient_alpha, readmode));
                    state.single_contour_clicked = false;
                }
                if state.show_other_topology_settings {
                    Window::new(&ImString::new(format!("Other Topology Settings")))
                        .size([300.0, 100.0], Condition::Appearing)
                        .resizable(false)
                        .build(self, || {
                            Slider::new(format!("Interval"), 1.0, 10.0)
                                .build(self, &mut state.topology_interval);
                        });
                }
                if state.show_colormapedit {
                    const BG_COLOR: u32 = 0xA033_3333;
                    const LINE_COLORS: [u32; 3] = [0xFF00_00FF, 0xFF00_FF00, 0xFFFF_0000];
                    const MARGIN_TOP: f32 = 20.0;
                    const MARGIN_LEFT: f32 = 30.0;
                    const OFFSET_X: f32 = 60.0;
                    const OFFSET_Y: f32 = 120.0;
                    Window::new(&ImString::new(format!("Transfer Fucntion")))
                        .size([960.0, 720.0], Condition::Appearing)
                        .resizable(false)
                        .menu_bar(true)
                        .build(self, || {
                            self.menu_bar(|| {
                                if let Some(menu) =
                                    self.begin_menu_with_enabled(format!("Preset"), true)
                                {
                                    for builtin_lut in BuiltinLUT::values() {
                                        if MenuItem::new(builtin_lut.name()).build(self) {
                                            state.lut.set_gradient(*builtin_lut);
                                        }
                                    }
                                    menu.end();
                                }
                            });

                            let p = self.cursor_screen_pos();
                            let size = self.window_content_region_max();
                            let draw_list = self.get_window_draw_list();
                            draw_list.add_text(
                                p,
                                0xFFFF_FFFF,
                                &format!("Graph color Mode: {:?}", state.lut.read_mode()),
                            );
                            /*draw_list.with_clip_rect_intersect(
                            [p[0] + 30.0, p[1] + 20.0],
                            [p[0] + size[0], p[1] + size[1]],
                            || {*/
                            draw_list
                                .add_rect(
                                    [p[0] + MARGIN_LEFT, p[1] + MARGIN_TOP],
                                    [
                                        p[0] + MARGIN_LEFT + size[0] - OFFSET_X,
                                        p[1] + MARGIN_TOP + size[1] - OFFSET_Y,
                                    ],
                                    BG_COLOR,
                                )
                                .filled(true)
                                .build();
                            let mut gradient = state.lut.gradient();
                            let first = gradient.iter();
                            let second = gradient.iter().skip(1);

                            for (lut1, lut2) in first.zip(second) {
                                for c in 0..3 {
                                    let x1 = p[0] + MARGIN_LEFT + (size[0] - OFFSET_X) * lut1.0;
                                    let x2 = p[0] + MARGIN_LEFT + (size[0] - OFFSET_X) * lut2.0;
                                    let y1 = p[1]
                                        + MARGIN_TOP
                                        + (255.0 - lut1.1[c] as f32) / 255.0 * (size[1] - OFFSET_Y);
                                    let y2 = p[1]
                                        + MARGIN_TOP
                                        + (255.0 - lut2.1[c] as f32) / 255.0 * (size[1] - OFFSET_Y);
                                    draw_list
                                        .add_line([x1, y1], [x2, y2], LINE_COLORS[c])
                                        .build();
                                }
                            }
                            let gradient_alpha = state.lut.gradient_alpha();
                            let first = gradient_alpha.iter();
                            let second = gradient_alpha.iter().skip(1);

                            for (a1, a2) in first.zip(second) {
                                let x1 = p[0] + MARGIN_LEFT + (size[0] - OFFSET_X) * a1.0;
                                let x2 = p[0] + MARGIN_LEFT + (size[0] - OFFSET_X) * a2.0;
                                let y1 = p[1]
                                    + MARGIN_TOP
                                    + (255.0 - a1.1 as f32) / 255.0 * (size[1] - OFFSET_Y);
                                let y2 = p[1]
                                    + MARGIN_TOP
                                    + (255.0 - a2.1 as f32) / 255.0 * (size[1] - OFFSET_Y);
                                draw_list.add_line([x1, y1], [x2, y2], 0xFFFF_FFFF).build();
                            }

                            let mut lut_changed = false;
                            draw_list
                                .add_line(
                                    [
                                        p[0] + MARGIN_LEFT,
                                        p[1] + MARGIN_TOP + size[1] - OFFSET_Y + 33.0,
                                    ],
                                    [
                                        p[0] + MARGIN_LEFT + (size[0] - OFFSET_X),
                                        p[1] + MARGIN_TOP + size[1] - OFFSET_Y + 33.0,
                                    ],
                                    0xFFFF_FFFF,
                                )
                                .build();
                            let x = p[0] + MARGIN_LEFT;
                            let y = p[1] + MARGIN_TOP + size[1] - OFFSET_Y + 25.0;
                            self.set_cursor_screen_pos([x, y]);
                            self.invisible_button(
                                format!("tf_control_bar"),
                                [size[0] - OFFSET_X, 13.0],
                            );
                            let mouse_pos = self.io().mouse_pos;
                            if self.is_item_hovered() && self.is_mouse_clicked(MouseButton::Right) {
                                let mouse_x = mouse_pos[0];
                                let v = (mouse_x - p[0]) / (size[0] - OFFSET_X);
                                let prev = gradient.iter();
                                let next = gradient.iter().skip(1);
                                let mut insert_pos = 1;
                                let mut r = 0;
                                let mut g = 0;
                                let mut b = 0;
                                for (p, n) in prev.zip(next) {
                                    if p.0 < v && v < n.0 {
                                        let readmode = state.lut.read_mode();
                                        let coef = (v - p.0) / (n.0 - p.0);
                                        match readmode {
                                            lut::ReadMode::RGB | lut::ReadMode::HSV => {
                                                let r1 = p.1[0] as f32;
                                                let r2 = n.1[0] as f32;
                                                let g1 = p.1[1] as f32;
                                                let g2 = n.1[1] as f32;
                                                let b1 = p.1[2] as f32;
                                                let b2 = n.1[2] as f32;
                                                r = (r1 + (r2 - r1) * coef) as u8;
                                                g = (g1 + (g2 - g1) * coef) as u8;
                                                b = (b1 + (b2 - b1) * coef) as u8;
                                            }
                                            lut::ReadMode::LAB => {
                                                let pi = std::f32::consts::PI;
                                                let l1 = f32::from(p.1[0]);
                                                let l2 = f32::from(n.1[0]);
                                                let a1 = (f32::from(p.1[1]) - 128.0) / 128.0;
                                                let a2 = (f32::from(n.1[1]) - 128.0) / 128.0;
                                                let b1 = (f32::from(p.1[2]) - 128.0) / 128.0;
                                                let b2 = (f32::from(n.1[2]) - 128.0) / 128.0;
                                                let rad1 = b1.atan2(a1);
                                                let rad2 = b2.atan2(a2);
                                                let rad1 =
                                                    if rad1 < 0.0 { rad1 + 2.0 * pi } else { rad1 };
                                                let rad2 =
                                                    if rad2 < 0.0 { rad2 + 2.0 * pi } else { rad2 };
                                                let rad = rad1 + (rad2 - rad1) * coef;
                                                let la = (rad.cos() * 128.0 + 128.0) as u8;
                                                let lb = (rad.sin() * 128.0 + 128.0) as u8;

                                                [r, g, b] = [(l1 + (l2 - l1) * coef) as u8, la, lb];
                                            }
                                        }
                                        break;
                                    }
                                    insert_pos += 1;
                                }
                                state.lut_color_adding = Some((insert_pos, (v, [r, g, b])));
                                self.open_popup("tf_control");
                            }
                            self.popup("tf_control", || {
                                if MenuItem::new("Add new").build(self) {
                                    if let Some((k, c)) = state.lut_color_adding {
                                        gradient.insert(k, c);
                                        lut_changed = true;
                                    }
                                }
                            });
                            let gradient_size = gradient.len();
                            for (k, (v, color)) in gradient.iter_mut().enumerate() {
                                let color_rgb = match state.lut.read_mode() {
                                    lut::ReadMode::HSV => state.lut.hsv2rgb(*color),
                                    lut::ReadMode::LAB => state.lut.lab2rgb(*color),
                                    lut::ReadMode::RGB => *color,
                                };
                                let mut color_float = [
                                    color_rgb[0] as f32 / 255.0,
                                    color_rgb[1] as f32 / 255.0,
                                    color_rgb[2] as f32 / 255.0,
                                ];
                                let x = p[0] + MARGIN_LEFT + (size[0] - OFFSET_X) * (*v);
                                let y = p[1] + MARGIN_TOP + size[1] - OFFSET_Y + 30.0;
                                self.set_cursor_screen_pos([x - 9.0, y - 6.0]);
                                if ColorEdit::new(format!("##{}", k), &mut color_float)
                                    .inputs(false)
                                    .build(self)
                                {
                                    let edited_color = &[
                                        (color_float[0] * 255.0) as u8,
                                        (color_float[1] * 255.0) as u8,
                                        (color_float[2] * 255.0) as u8,
                                    ];
                                    *color = *edited_color;
                                    lut_changed = true;
                                }
                                if self.is_item_hovered() && 0 < k && k < gradient_size - 1 {
                                    self.set_mouse_cursor(Some(MouseCursor::ResizeEW));
                                    if self.is_mouse_clicked(MouseButton::Left) {
                                        state.lut_any_moving = Some(k);
                                    } else if self.is_mouse_clicked(MouseButton::Right) {
                                        state.lut_color_deleting = Some(k);
                                        self.open_popup("delete-color");
                                    }
                                }
                            }
                            if let Some(c) = state.lut_any_moving {
                                let mouse_x = mouse_pos[0];
                                let mut v = (mouse_x - p[0] - MARGIN_LEFT) / (size[0] - OFFSET_X);
                                let prev_v = gradient.get(c as usize - 1).unwrap().0;
                                let next_v = gradient.get(c as usize + 1).unwrap().0;
                                if v < prev_v {
                                    v = prev_v;
                                } else if v > next_v {
                                    v = next_v;
                                }
                                gradient[c].0 = v;
                                lut_changed = true;
                            }
                            self.popup("delete-color", || {
                                if MenuItem::new("Delete Color").build(self) {
                                    if let Some(k) = state.lut_color_deleting {
                                        gradient.remove(k);
                                        lut_changed = true;
                                    }
                                }
                            });
                            if lut_changed {
                                state.lut.set_gradient((
                                    gradient,
                                    gradient_alpha,
                                    state.lut.read_mode(),
                                ));
                            }
                            if !self.is_mouse_down(MouseButton::Left) {
                                state.lut_any_moving = None;
                            }
                            //},
                            //);
                        });
                }
                Image::new(texture_id, size).build(self);
                if self.is_item_hovered() {
                    if self.is_mouse_down(MouseButton::Left) {
                        if state.mouse_moving {
                            let mousenow_pos = self.io().mouse_pos;
                            let [dx, dy] = [
                                mousenow_pos[0] - state.mousedown_pos[0],
                                mousenow_pos[1] - state.mousedown_pos[1],
                            ];
                            let dtheta = dx / childwindow_size[0] * 2.0 * std::f32::consts::PI;
                            let dphi = dy / childwindow_size[1] * 2.0 * std::f32::consts::PI;
                            state.theta += dtheta;
                            state.phi += dphi;
                            state.mousedown_pos = mousenow_pos;
                        } else {
                            state.mousedown_pos = self.io().mouse_pos;
                            state.mouse_moving = true;
                        }
                    } else {
                        if state.mouse_moving {
                            state.mousedown_pos = [0.0, 0.0];
                            state.mouse_moving = false;
                        }
                    }
                    state.mouse_wheel_delta = self.io().mouse_wheel;
                } else {
                    if state.mouse_moving {
                        state.mousedown_pos = [0.0, 0.0];
                        state.mouse_moving = false;
                    }
                }
            });
    }
}

#[derive(Copy, Clone)]
struct Vertex {
    pos: [f32; 3],
    texcoord: [f32; 2],
}
implement_vertex!(Vertex, pos, texcoord);

fn ray_casting_gpu<S>(
    image: &ArrayBase<S, Ix3>,
    topology: &Option<Topology>,
    n: usize,
    m: usize,
    state: &mut State,
) -> Vec<u8>
where
    S: Data<Elem = f32>,
{
    let mut data = vec![0; 3 * n * m];
    let theta = state.theta;
    let phi = state.phi;
    state.eyepos_z += state.mouse_wheel_delta * 0.1;
    if state.eyepos_z > 0.0 {
        state.eyepos_z = 0.0;
    } else if state.eyepos_z < -10.0 {
        state.eyepos_z = -10.0;
    }
    let eyepos_z = state.eyepos_z;

    let [nx1, ny1, nz1] = state.axis1;
    let [nx2, ny2, nz2] = state.axis2;
    let rmat1 = [
        [
            nx1 * nx1 * (1.0 - theta.cos()) + theta.cos(),
            nx1 * ny1 * (1.0 - theta.cos()) + nz1 * theta.sin(),
            nx1 * nz1 * (1.0 - theta.cos()) - ny1 * theta.sin(),
            0.0,
        ],
        [
            nx1 * ny1 * (1.0 - theta.cos()) - nz1 * theta.sin(),
            ny1 * ny1 * (1.0 - theta.cos()) + theta.cos(),
            ny1 * nz1 * (1.0 - theta.cos()) + nx1 * theta.sin(),
            0.0,
        ],
        [
            nx1 * nz1 * (1.0 - theta.cos()) + ny1 * theta.sin(),
            ny1 * nz1 * (1.0 - theta.cos()) - nx1 * theta.sin(),
            nz1 * nz1 * (1.0 - theta.cos()) + theta.cos(),
            0.0,
        ],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let rmat2 = [
        [
            nx2 * nx2 * (1.0 - phi.cos()) + phi.cos(),
            nx2 * ny2 * (1.0 - phi.cos()) + nz2 * phi.sin(),
            nx2 * nz2 * (1.0 - phi.cos()) - ny2 * phi.sin(),
            0.0,
        ],
        [
            nx2 * ny2 * (1.0 - phi.cos()) - nz2 * phi.sin(),
            ny2 * ny2 * (1.0 - phi.cos()) + phi.cos(),
            ny2 * nz2 * (1.0 - phi.cos()) + nx2 * phi.sin(),
            0.0,
        ],
        [
            nx2 * nz2 * (1.0 - phi.cos()) + ny2 * phi.sin(),
            ny2 * nz2 * (1.0 - phi.cos()) - nx2 * phi.sin(),
            nz2 * nz2 * (1.0 - phi.cos()) + phi.cos(),
            0.0,
        ],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let display = state.display.clone().unwrap();

    let program = glium::Program::from_source(
        display.get_context(),
        "#version 130
        in vec3 pos;
        in vec2 texcoord;
        out vec2 Texcoord;
        void main() {
          gl_Position = vec4(pos, 1.0);
          Texcoord = texcoord;
        }",
        "#version 130
        precision highp float;
        #define PI (3.14159265359)
        #define FLT_MAX (3.402823466e+38)

        in vec2 Texcoord;
        out vec4 out_color;
        uniform sampler3D volume;
        uniform mat4 rmat1;
        uniform mat4 rmat2;
        uniform float eyepos_z;
        uniform float cp_size;
        uniform highp sampler1D cp_vals;
        uniform highp sampler1D color_lut;
        uniform float brightness;
        uniform float upperbound;
        uniform int render_mode;

        struct ray {
            vec3 origin;
            vec3 direction;
            vec4 color;
        };

        vec4 hsv2rgb(const in vec4 hsv){
            float h = hsv.r * 360.0;
            float s = hsv.g * 255.0;
            float v = hsv.b * 255.0;
            float max = v;
            float min = max - (hsv.g * max);
            vec4 result = vec4(0.0f);
            if (0.0 <= h && h < 60.0) {
                float g = (h / 60.0) * (max - min) + min;
                result = vec4(max / 255.0f, g / 255.0f, min / 255.0f, 0.0f);
            } else if (60.0 <= h && h < 120.0) {
                float r = ((120.0 - h) / 60.0) * (max - min) + min;
                result = vec4(r / 255.0f, max / 255.0f, min / 255.0f, 0.0f);
            } else if (120.0 <= h && h < 180.0) {
                float b = ((h - 120.0) / 60.0) * (max - min) + min;
                result = vec4(min / 255.0f, max / 255.0f, b / 255.0f, 0.0f);
            } else if (180.0 <= h && h < 240.0) {
                float g = ((240.0 - h) / 60.0) * (max - min) + min;
                result = vec4(min / 255.0f, g / 255.0f, max / 255.0f, 0.0f);
            } else if (240.0 <= h && h < 300.0) {
                float r = ((h - 240.0) / 60.0) * (max - min) + min;
                result = vec4(r / 255.0f, min / 255.0f, max / 255.0f, 0.0f);
            } else if (300.0 <= h && h < 360.0) {
                float b = ((360.0 - h) / 60.0) * (max - min) + min;
                result = vec4(max / 255.0f, min / 255.0f, b / 255.0f, 0.0f);
            } else {
                result = vec4(0.0f, 0.0f, 0.0f, 0.0f);
            }
            return result;
        }

        bool hit_volume(inout ray r) {
            const int nDim = 3;
            const vec3 _min = vec3(-1.0f, -1.0f, -1.0f);
            const vec3 _max = vec3(1.0f, 1.0f, 1.0f);
            float tmin = 0.0f;
            float tmax = FLT_MAX;
            float t0;
            float t1;
            for (int i = 0; i < nDim; i++) {
                t0 = min((_min[i] - r.origin[i]) / r.direction[i],
                        (_max[i] - r.origin[i]) / r.direction[i]);
                t1 = max((_min[i] - r.origin[i]) / r.direction[i],
                        (_max[i] - r.origin[i]) / r.direction[i]);
                tmin = max(t0, tmin);
                tmax = min(t1, tmax);
                if (tmax <= tmin) {
                        return false;
                }
            }
            r.origin = r.origin + tmin * r.direction;
            return true;
        }

        vec4 color_legend(const in float val) {
            float temp = (-cos(4.0f * val * PI) + 1.0f) / 2.0f;
            vec4 result =
                (val > 1.0f) ? vec4(1.0f, 0.0f, 0.0f, 0.0f) :
                (val > 3.0f / 4.0f) ? vec4(1.0f, temp, 0.0f, 0.0f) * val :
                (val > 2.0f / 4.0f) ? vec4(temp, 1.0f, 0.0f, 0.0f) * val :
                (val > 1.0f / 4.0f) ? vec4(0.0f, 1.0f, temp, 0.0f) * val :
                (val > 0.0f) ? vec4(0.0f, temp, 1.0f, 0.0f) * val : vec4(0.0f, 0.0f, 0.0f, 0.0f);

            return result;
        }

        vec4 color_legend_from_lut(const in float val, inout ray re) {
            float r = texture(color_lut, float(val)).r;
            float g = texture(color_lut, float(val)).g;
            float b = texture(color_lut, float(val)).b;
            float E = 2.71828;
            float a = texture(color_lut, float(val)).a;
            int cp_size_int = int(cp_size);
            float distance = sqrt(re.origin.x * re.origin.x + re.origin.y * re.origin.y + re.origin.z * re.origin.z);
            if (render_mode == 1) {
                a *= brightness;
            } else {
                a *= brightness*pow(E, -distance*distance);
            }
            return vec4(r, g, b, a);
        }

        float alpha_legend_topological(const in float val) {
            float min_span = FLT_MAX;
            float distance = 0.0;
            int cp_size_int = int(cp_size);
            float result = 0.0;
            for (int i = 1; i < cp_size_int-1; i++) {
                float val1 = texture(cp_vals, float(i) / float(cp_size_int)).r;
                float val2 = texture(cp_vals, float(i + 1) / float(cp_size_int)).r;
                if ((val2 - val1) / 3.0 > 0.0) {
                    min_span = min(min_span, (val2 - val1) / 3.0);
                }
            }
            for (int i = 0; i < cp_size_int; i++) {
                float val1 = texture(cp_vals, float(i) / float(cp_size_int)).r;
                float val2 = texture(cp_vals, float(i + 1) / float(cp_size_int)).r;
                if (val1 <= val && val <= val2) {
                    distance = min(val2 - val, val - val1);
                    break;
                }
            }
            if (distance > min_span) {
                result = 0.2f;
            } else {
                result = 0.2f+(min_span - distance) / min_span * (val * brightness);
                if (result > upperbound) {
                    result = upperbound;
                }
            }
            return result;
        }

        vec4 color_legend_topological(const in float val) {
            int counter = 0;
            int cp_size_int = int(cp_size);
            float down_hue = (2.0 / 3.0) / cp_size_int;
            float ratio = 0.0;
            if (val > 1.0) {
                return vec4(1.0f, 0.0f, 0.0f, 0.0f);
            } else if (val <= 0.0) {
                return vec4(0.0f, 0.0f, 0.0f, 0.0f);
            }

            for (int i = 0; i < cp_size_int; i++) {
                float val1 = texture(cp_vals, float(i) / float(cp_size_int)).r;
                float val2 = texture(cp_vals, float(i + 1) / float(cp_size_int)).r;
                if (val1 <= val && val <= val2) {
                    ratio = (val - val1) / (val2 - val1);
                    break;
                } else {
                    counter++;
                }
            }
            float hue = (2.0 / 3.0) - down_hue * counter - down_hue * ratio;
            vec4 hsv = vec4(hue, 1.0, 1.0, 0.0);
            vec4 rgb = hsv2rgb(hsv);
            
            return rgb;
        }

        void sampling_volume(inout ray r) {
            const float dt = 1.0f / 2000.0f;
            int step = 1;
            int cp_size_int = int(cp_size);
            float val;
            while (hit_volume(r))
            {
                float dis = sqrt(r.origin.x * r.origin.x + r.origin.y * r.origin.y * r.origin.z * r.origin.z);
                float dis_s = 1.0f / (dis + 1.0f);
                val = texture(volume, r.origin / 2.0f + vec3(0.5)).r;
                if (cp_size_int <= 2) {
                    r.color += (color_legend(val) - r.color) / step++;
                    r.color.a = 1.0f;
                } else {
                    r.color += (color_legend_from_lut(val, r) - r.color) / step++;
                    //r.color.a += ((alpha_legend_topological(val) - r.color.a) / step++) * dis_s;
                }
                r.origin += r.direction * dt;
            }
        }

        vec4 gammaCorrect(const in vec4 color, const in float gamma) {
            float g = 1.0f / gamma;
            vec4 result = vec4
            (
                pow(color.r, g),
                pow(color.g, g),
                pow(color.b, g),
                color.a
            );
            if (color.r == 0.0f && color.g == 0.0f && color.b == 0.0f) {
                result = vec4(0.0f, 0.0f, 0.0f, 0.0f);
            }
            return result;
        }

        

        vec4 jetcolor(const in float val) {
            vec4 hsv = vec4((1.0f - val) * (2.0f / 3.0f), 1.0f, 1.0f, 1.0f);
            vec4 rgb = hsv2rgb(hsv);
            return rgb;
        }

        void main() {
            vec4 eye = vec4(0.0f, 0.0f, eyepos_z, 1.0f);
            vec4 position_screen = vec4
            (
                Texcoord.x * 2.0 - 1.0,
                Texcoord.y * 2.0 - 1.0,
                eye.z + 0.5f,
                1.0f
            );
            /*const mat3 M1 = mat3(
                cos(-theta), 0, sin(-theta),
                0, 1, 0,
                -sin(-theta), 0, cos(-theta)
            );
            const mat3 M2 = mat3(
                1, 0, 0,
                0, cos(-phi), -sin(-phi),
                0, sin(-phi), cos(-phi)
            );*/

            ray r;
            r.origin = (rmat1 * rmat2 * eye).xyz;
            r.direction = (normalize(rmat1 * rmat2 * (position_screen - eye))).xyz;
            r.color = vec4(0.0f);

            sampling_volume(r);
            out_color = gammaCorrect(r.color, 2.2);


            int cp_size_int = int(cp_size);
            float min_span = 0.0f;
            for (int i = 1; i < cp_size_int-1; i++) {
                highp float val1 = texture(cp_vals, float(i) / float(cp_size_int)).r;
                highp float val2 = texture(cp_vals, float(i + 1) / float(cp_size_int)).r;
                if (val1 == val2) {
                    out_color = vec4(float(i) / float(cp_size_int), float(i) / float(cp_size_int), float(i) / float(cp_size_int), 1.0f);
                    break;
                }
            }
            /*float val = texture(cp_vals, Texcoord.x).r;
            out_color = jetcolor(val);
            out_color.a = 1.0f;
            highp float val1 = 0.017067295;
            highp float val2 = 0.017032975;
            if (val2 - val1 == 0.0) {
                out_color = vec4(1.0f, 1.0f, 1.0f, 1.0f);
            }*/
        }",
        None,
    )
    .unwrap();

    let fb_tex = Texture2d::empty_with_format(
        display.get_context(),
        glium::texture::UncompressedFloatFormat::F32F32F32F32,
        glium::texture::MipmapsOption::NoMipmap,
        n as u32,
        m as u32,
    )
    .unwrap();
    let mut fb =
        glium::framebuffer::SimpleFrameBuffer::new(display.get_context(), &fb_tex).unwrap();

    let vertex_buffer = glium::VertexBuffer::new(
        display.get_context(),
        &[
            Vertex {
                pos: [1.0, -1.0, 0.0],
                texcoord: [1.0, 1.0],
            },
            Vertex {
                pos: [-1.0, -1.0, 0.0],
                texcoord: [0.0, 1.0],
            },
            Vertex {
                pos: [-1.0, 1.0, 0.0],
                texcoord: [0.0, 0.0],
            },
            Vertex {
                pos: [1.0, 1.0, 0.0],
                texcoord: [1.0, 0.0],
            },
        ],
    )
    .unwrap();
    let index_buffer = glium::index::NoIndices(glium::index::PrimitiveType::TriangleFan);
    let mut shape = image.dim();
    if shape.0 > 128 {
        shape.0 = 128;
    } // max texture size is 2048
    let mut volume_data = vec![vec![vec![0f32; shape.2]; shape.1]; shape.0];
    let mut min_val = std::f32::MAX;
    let mut max_val = 0f32;
    for i in 0..shape.0 {
        for j in 0..shape.1 {
            for k in 0..shape.2 {
                if min_val > image[[i, j, k]] {
                    min_val = if image[[i, j, k]] < 0f32 {
                        0f32
                    } else {
                        image[[i, j, k]]
                    };
                }
                if max_val < image[[i, j, k]] {
                    max_val = image[[i, j, k]];
                }
            }
        }
    }
    for i in 0..shape.0 {
        for j in 0..shape.1 {
            for k in 0..shape.2 {
                volume_data[i][j][k] = (if image[[i, j, k]] < 0f32 {
                    0f32
                } else {
                    image[[i, j, k]]
                } - min_val)
                    / (max_val - min_val);
            }
        }
    }
    let mut cp_vals = vec![0.0];
    if let Some(topology) = topology {
        for cp in &topology.critical_points {
            let (mut x, mut y, mut z) = cp.coord;
            if x >= shape.1 as f32 {
                x = (shape.1 - 1) as f32;
            }
            if y >= shape.2 as f32 {
                y = (shape.2 - 1) as f32;
            }
            if z >= shape.0 as f32 {
                z = (shape.0 - 1) as f32;
            }
            if cp.manifoldsize > 0 && !volume_data[z as usize][x as usize][y as usize].is_nan() {
                cp_vals.push(volume_data[z as usize][x as usize][y as usize]);
            }
        }
    }
    cp_vals.push(1.0);
    cp_vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    cp_vals.dedup_by(|a, b| (*a - *b).abs() < 1.0e-2);
    let first = cp_vals.iter();
    let second = cp_vals.iter().skip(1);
    for (c1, c2) in first.zip(second) {
        if *c1 == *c2 {
            println!("true");
        }
    }
    let texture = glium::texture::CompressedTexture3d::new(
        display.get_context(),
        glium::texture::Texture3dDataSource::into_raw(volume_data),
    )
    .unwrap();
    //println!("{:?}", cp_vals);
    let cp_size = cp_vals.len();
    let cp_tex = glium::texture::Texture1d::new(
        display.get_context(),
        glium::texture::Texture1dDataSource::into_raw(cp_vals),
    )
    .unwrap();
    let mut color_lut = vec![];
    for i in 0..256 {
        color_lut.push(state.lut.color_at(i as f32 / 255.0)[0] as f32 / 255.0);
        color_lut.push(state.lut.color_at(i as f32 / 255.0)[1] as f32 / 255.0);
        color_lut.push(state.lut.color_at(i as f32 / 255.0)[2] as f32 / 255.0);
        color_lut.push(state.lut.color_at(i as f32 / 255.0)[3] as f32 / 255.0);
    }
    let color_tex = glium::texture::Texture1d::with_format(
        display.get_context(),
        glium::texture::RawImage1d::from_raw_rgba(color_lut),
        glium::texture::UncompressedFloatFormat::F32F32F32F32,
        glium::texture::MipmapsOption::NoMipmap,
    )
    .unwrap();
    let render_mode = if state.show_single_contour { 1 } else { 0 };
    let uniforms = uniform! {
        volume: texture.sampled().magnify_filter(glium::uniforms::MagnifySamplerFilter::Linear),
        rmat1: rmat1, rmat2: rmat2, eyepos_z: eyepos_z, cp_size: cp_size as f32, cp_vals: cp_tex.sampled().magnify_filter(glium::uniforms::MagnifySamplerFilter::Linear),
        color_lut: color_tex.sampled()
    , brightness: state.topology_brightness, upperbound: state.topology_upperbound, render_mode: render_mode};
    let params = glium::DrawParameters {
        blend: glium::draw_parameters::Blend::alpha_blending(),
        ..Default::default()
    };
    fb.draw(&vertex_buffer, &index_buffer, &program, &uniforms, &params)
        .unwrap();
    draw_axes(state, &mut fb);

    let read_back: Vec<Vec<(u8, u8, u8, u8)>> = fb_tex.read();
    for i in 0..n {
        for j in 0..m {
            data[(i * m + j) * 3 + 0] = read_back[i][j].0;
            data[(i * m + j) * 3 + 1] = read_back[i][j].1;
            data[(i * m + j) * 3 + 2] = read_back[i][j].2;
        }
    }

    data
}

fn make_raw_image<S>(
    image: &ArrayBase<S, IxDyn>,
    topology: &Option<Topology>,
    state: &mut State,
) -> RawImage2d<'static, u8>
where
    S: Data<Elem = f32>,
{
    let image3 = image.slice(s![.., .., ..]);
    let n = 512;
    let m = 512;
    let data = ray_casting_gpu(&image3, topology, n, m, state);
    RawImage2d {
        data: Cow::Owned(data),
        width: n as u32,
        height: m as u32,
        format: ClientFormat::U8U8U8,
    }
}

fn draw_axes(state: &State, fb: &mut glium::framebuffer::SimpleFrameBuffer) {
    let theta = state.theta;
    let phi = state.phi;
    let display = state.display.clone().unwrap();
    const AXIS_LEN: f32 = 0.3;
    let line_x: [Vertex; 2] = [
        Vertex {
            pos: [0.0, 0.0, 0.0],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [AXIS_LEN, 0.0, 0.0],
            texcoord: [1.0, 1.0],
        },
    ];
    let line_x = glium::VertexBuffer::new(&display, &line_x).unwrap();
    let line_y: [Vertex; 2] = [
        Vertex {
            pos: [0.0, 0.0, 0.0],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [0.0, AXIS_LEN, 0.0],
            texcoord: [1.0, 1.0],
        },
    ];
    let line_y = glium::VertexBuffer::new(&display, &line_y).unwrap();

    let line_z: [Vertex; 2] = [
        Vertex {
            pos: [0.0, 0.0, 0.0],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [0.0, 0.0, AXIS_LEN],
            texcoord: [1.0, 1.0],
        },
    ];
    let line_z = glium::VertexBuffer::new(&display, &line_z).unwrap();
    let line_indices = glium::index::NoIndices(glium::index::PrimitiveType::LinesList);

    let vertex_shader_src = r#"
        #version 130
        in vec3 pos;
        in vec2 texcoord;
        out vec3 col;
        uniform vec3 color;
        uniform float theta;
        uniform float phi;
        void main() {
            mat3 M1 = mat3(
                cos(-theta), 0, sin(-theta),
                0, 1, 0,
                -sin(-theta), 0, cos(-theta)
            );
            mat3 M2 = mat3(
                1, 0, 0,
                0, cos(-phi), -sin(-phi),
                0, sin(-phi), cos(-phi)
            );
            gl_Position = vec4(M2*M1*pos / 6.0, 1.0);
            col = color;
        }
    "#;

    let fragment_shader_src = r#"
        #version 130
        in vec3 col;
        out vec4 color;
        void main() {
            color = vec4(col, 1);
        }
    "#;
    let program_axes = glium::Program::from_source(
        display.get_context(),
        vertex_shader_src,
        fragment_shader_src,
        None,
    )
    .unwrap();

    fb.draw(
        &line_x,
        &line_indices,
        &program_axes,
        &uniform! {color: [1.0f32, 0.0, 0.0], theta: theta, phi: phi},
        &Default::default(),
    )
    .unwrap();
    fb.draw(
        &line_y,
        &line_indices,
        &program_axes,
        &uniform! {color: [0.0, 1.0f32, 0.0], theta: theta, phi: phi},
        &Default::default(),
    )
    .unwrap();
    fb.draw(
        &line_z,
        &line_indices,
        &program_axes,
        &uniform! {color: [0.0, 0.0, 1.0f32], theta: theta, phi: phi},
        &Default::default(),
    )
    .unwrap();
}

fn _draw_line(
    state: &mut State,
    fb: &mut glium::framebuffer::SimpleFrameBuffer,
    coord1: [f32; 3],
    coord2: [f32; 3],
    color: [f32; 3],
) {
    let display = state.display.clone().unwrap();
    let theta = state.theta;
    let phi = state.phi;
    let eyepos_z = state.eyepos_z;
    let line: [Vertex; 2] = [
        Vertex {
            pos: coord1,
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: coord2,
            texcoord: [0.0, 0.0],
        },
    ];
    let line_positions = glium::VertexBuffer::new(&display, &line).unwrap();
    let line_indices = glium::index::NoIndices(glium::index::PrimitiveType::LinesList);
    let vertex_shader_src = r#"
        #version 430
        in vec3 pos;
        in vec2 texcoord;
        out vec3 col;
        uniform vec3 color;
        uniform float theta;
        uniform float phi;
        uniform float eyepos_z;
        void main() {
            const mat3 M1 = mat3(
                cos(-theta), 0, sin(-theta),
                0, 1, 0,
                -sin(-theta), 0, cos(-theta)
            );
            const mat3 M2 = mat3(
                1, 0, 0,
                0, cos(-phi), -sin(-phi),
                0, sin(-phi), cos(-phi)
            );
            vec4 posr = vec4(M2 * M1 * pos, 1.0);
            gl_Position.x = posr.x*0.5 / (posr.z + -eyepos_z);
            gl_Position.y = posr.y*0.5 / (posr.z + -eyepos_z);
            gl_Position.z = posr.z;
            gl_Position = vec4(gl_Position.xyz, 1.0);
            col = color;
        }
    "#;
    let fragment_shader_src = r#"
        #version 430
        in vec3 col;
        out vec4 color;
        void main() {
            color = vec4(col, 1);
        }
    "#;
    let program_line = glium::Program::from_source(
        display.get_context(),
        vertex_shader_src,
        fragment_shader_src,
        None,
    )
    .unwrap();
    fb.draw(
        &line_positions,
        &line_indices,
        &program_line,
        &uniform! {color: color, theta: theta, phi: phi, eyepos_z: eyepos_z},
        &Default::default(),
    )
    .unwrap();
}

fn _draw_pyramid(
    state: &mut State,
    fb: &mut glium::framebuffer::SimpleFrameBuffer,
    coord: [f32; 3],
    scale: f32,
    color: [f32; 3],
) {
    let display = state.display.clone().unwrap();
    let eyepos_z = state.eyepos_z;
    let boxline: [Vertex; 16] = [
        Vertex {
            pos: [1.0, 1.0, -1.0],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [-1.0, 1.0, -1.0],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [1.0, 1.0, -1.0],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [1.0, -1.0, -1.0],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [-1.0, -1.0, -1.0],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [-1.0, 1.0, -1.0],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [-1.0, -1.0, -1.0],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [1.0, -1.0, -1.0],
            texcoord: [0.0, 0.0],
        },
        //split
        Vertex {
            pos: [1.0, 1.0, 1.0],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [-1.0, 1.0, 1.0],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [1.0, 1.0, 1.0],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [1.0, -1.0, 1.0],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [-1.0, -1.0, 1.0],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [-1.0, 1.0, 1.0],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [-1.0, -1.0, 1.0],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [1.0, -1.0, 1.0],
            texcoord: [0.0, 0.0],
        },
    ];
    let box_positions = glium::VertexBuffer::new(&display, &boxline).unwrap();
    let box_indices = glium::index::NoIndices(glium::index::PrimitiveType::LinesList);
    let vertex_shader_src_box = r#"
        #version 430
        in vec3 pos;
        in vec2 texcoord;
        out vec3 col;
        uniform vec3 color;
        uniform float theta;
        uniform float phi;
        uniform vec3 coord;
        uniform float scale;
        uniform float eyepos_z;
        void main() {
            const mat3 M1 = mat3(
                cos(-theta), 0, sin(-theta),
                0, 1, 0,
                -sin(-theta), 0, cos(-theta)
            );
            const mat3 M2 = mat3(
                1, 0, 0,
                0, cos(-phi), -sin(-phi),
                0, sin(-phi), cos(-phi)
            );
            vec4 posr = vec4(M2 * M1 * pos, 1.0);
            gl_Position.x = posr.x*0.5 / (posr.z + -eyepos_z);
            gl_Position.y = posr.y*0.5 / (posr.z + -eyepos_z);
            gl_Position.z = posr.z;
            gl_Position = vec4(gl_Position.xyz, 1.0);
            col = color;
        }
    "#;

    let theta = state.theta;
    let phi = state.phi;
    let pyramid_shape: [Vertex; 12] = [
        Vertex {
            pos: [0.0, 0.0, 0.0],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [1.0, 0.0, 0.0],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [0.5, 3.0f32.sqrt() / 2.0, 0.0],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [1.0, 0.0, 0.0],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [0.5, 3.0f32.sqrt() / 2.0, 0.0],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [0.5, 3.0f32.sqrt() / 6.0, (2.0 / 3.0 as f32).sqrt()],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [0.0, 0.0, 0.0],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [0.5, 3.0f32.sqrt() / 2.0, 0.0],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [0.5, 3.0f32.sqrt() / 6.0, (2.0 / 3.0 as f32).sqrt()],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [1.0, 0.0, 0.0],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [0.0, 0.0, 0.0],
            texcoord: [0.0, 0.0],
        },
        Vertex {
            pos: [0.5, 3.0f32.sqrt() / 6.0, (2.0 / 3.0 as f32).sqrt()],
            texcoord: [0.0, 0.0],
        },
    ];

    let pyramid_positions = glium::VertexBuffer::new(&display, &pyramid_shape).unwrap();
    let pyramid_indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

    let vertex_shader_src = r#"
        #version 430
        in vec3 pos;
        in vec2 texcoord;
        out vec3 col;
        uniform vec3 color;
        uniform float theta;
        uniform float phi;
        uniform vec3 coord;
        uniform float scale;
        uniform float eyepos_z;

        void main() {
            const mat3 M1 = mat3(
                cos(-theta), 0, sin(-theta),
                0, 1, 0,
                -sin(-theta), 0, cos(-theta)
            );
            const mat3 M2 = mat3(
                1, 0, 0,
                0, cos(-phi), -sin(-phi),
                0, sin(-phi), cos(-phi)
            );
            gl_Position = vec4(M2 * M1 * (pos + vec3(-0.5, -sqrt(3.0) / 6.0, -sqrt(6.0) / 9.0)) * scale, 1.0);
            vec4 posr = vec4(M2 * M1 * coord, 1.0);
            gl_Position.x += posr.x * 0.5 / (posr.z + -eyepos_z);
            gl_Position.y += posr.y * 0.5 / (posr.z + -eyepos_z);
            gl_Position.z += posr.z;
            gl_Position = vec4(gl_Position.xyz, 1.0);
            col = color;
        }
    "#;

    let fragment_shader_src = r#"
        #version 430
        in vec3 col;
        out vec4 color;
        void main() {
            color = vec4(col, 1);
        }
    "#;
    let program_pyramid = glium::Program::from_source(
        display.get_context(),
        vertex_shader_src,
        fragment_shader_src,
        None,
    )
    .unwrap();
    let program_box = glium::Program::from_source(
        display.get_context(),
        vertex_shader_src_box,
        fragment_shader_src,
        None,
    )
    .unwrap();
    fb.draw(
        &pyramid_positions,
        &pyramid_indices,
        &program_pyramid,
        &uniform! {color: color, theta: theta, phi: phi, coord: coord, scale: scale, eyepos_z: eyepos_z},
        &Default::default(),
    )
    .unwrap();
    fb.draw(
        &box_positions,
        &box_indices,
        &program_box,
        &uniform! {color: [1.0f32, 1.0f32, 1.0f32], theta: theta, phi: phi, coord: [0.0f32, 0.0f32, 0.0f32], scale: 1.0f32, eyepos_z: eyepos_z},
        &Default::default(),
    )
    .unwrap();
}
