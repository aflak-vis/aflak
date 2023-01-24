use super::lut::{BuiltinLUT, ColorLUT};
use std::time::Instant;

pub struct State {
    pub display: Option<glium::Display>,
    pub created_on: Option<Instant>,
    pub mouse_moving: bool,
    pub mousedown_pos: [f32; 2],
    pub mouse_wheel_delta: f32,
    pub eyepos_z: f32,
    pub theta: f32,
    pub phi: f32,
    pub axis1: [f32; 3],
    pub axis2: [f32; 3],
    pub show_tf_parameters: bool,
    pub show_colormapedit: bool,
    pub topology_brightness: f32,
    pub show_other_topology_settings: bool,
    pub topology_interval: f32,
    pub lut: ColorLUT,
    pub lut_any_moving: Option<usize>,
    pub lut_color_deleting: Option<usize>,
    pub lut_color_adding: Option<(usize, (f32, [u8; 3]))>,
    pub show_single_contour: bool,
    pub single_contour: f32,
    pub single_contour_clicked: bool,
    pub critical_isosurface: bool,
    pub representative_isosurface: bool,
}

impl Default for State {
    fn default() -> Self {
        State {
            display: None,
            created_on: None,
            mouse_moving: false,
            mousedown_pos: [0.0, 0.0],
            mouse_wheel_delta: 0.0,
            eyepos_z: -2.0,
            theta: 0.0,
            phi: 0.0,
            axis1: [0.0, -1.0, 0.0],
            axis2: [1.0, 0.0, 0.0],
            show_tf_parameters: false,
            show_colormapedit: false,
            topology_brightness: 100.0,
            show_other_topology_settings: false,
            topology_interval: 3.0,
            lut: BuiltinLUT::TurboColor.lut(),
            lut_any_moving: None,
            lut_color_deleting: None,
            lut_color_adding: None,
            show_single_contour: false,
            single_contour: 0.5,
            single_contour_clicked: false,
            critical_isosurface: false,
            representative_isosurface: false,
        }
    }
}

impl State {
    pub fn new(&mut self, created_on: Instant) {
        let events_loop = glium::glutin::event_loop::EventLoop::new();
        let wb = glium::glutin::window::WindowBuilder::new().with_visible(false);
        let cb = glium::glutin::ContextBuilder::new().with_vsync(true);
        let display =
            glium::Display::new(wb, cb, &events_loop).expect("Failed to initialize display");
        self.display = Some(display);
        self.created_on = Some(created_on);
    }

    pub fn image_created_on(&self) -> Option<Instant> {
        self.created_on
    }
}
