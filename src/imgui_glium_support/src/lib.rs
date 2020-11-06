extern crate clipboard;
extern crate glium;

extern crate imgui;
extern crate imgui_glium_renderer;
extern crate imgui_winit_support;

use glium::backend::{self, Facade};
use glium::glutin;
use glium::glutin::event::{Event, WindowEvent};
use glium::glutin::event_loop::{ControlFlow, EventLoop};
use glium::glutin::window::WindowBuilder;
use glium::{Display, Surface};
use imgui::{Context, FontConfig, FontSource, Textures, Ui};
use imgui_glium_renderer::{Renderer, Texture};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;

mod clipboard_support;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub title: String,
    pub clear_color: [f32; 4],
    pub ini_filename: Option<PathBuf>,
    pub log_filename: Option<PathBuf>,
    pub window_width: u32,
    pub window_height: u32,
    pub maximized: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            title: "Default title".to_string(),
            clear_color: [1.0, 1.0, 1.0, 1.0],
            ini_filename: None,
            log_filename: None,
            window_width: 1024,
            window_height: 768,
            maximized: false,
        }
    }
}

pub struct System {
    pub event_loop: EventLoop<()>,
    pub display: glium::Display,
    pub imgui: Context,
    pub platform: WinitPlatform,
    pub renderer: Renderer,
    pub font_size: f32,
<<<<<<< HEAD
    pub clear_color: [f32; 4],
=======
>>>>>>> 916149c ([imgui_glium_support] Make this crate build (without considering error handling))
}

pub fn init(config: AppConfig) -> System {
    let title = match config.title.rfind('/') {
        Some(idx) => config.title.split_at(idx + 1).1,
        None => config.title.as_str(),
    };
    let event_loop = EventLoop::new();
    let context = glutin::ContextBuilder::new().with_vsync(true);
    let builder = WindowBuilder::new()
        .with_title(title.to_owned())
        .with_inner_size(glutin::dpi::LogicalSize::new(
            config.window_width as f64,
            config.window_height as f64,
        ))
        .with_maximized(config.maximized);
    let display =
        Display::new(builder, context, &event_loop).expect("Failed to initialize display");

    let mut imgui = Context::create();
    imgui.set_ini_filename(config.ini_filename);
    imgui.set_log_filename(config.log_filename);

    if let Some(backend) = clipboard_support::init() {
        imgui.set_clipboard_backend(Box::new(backend));
    } else {
        eprintln!("Failed to initialize clipboard");
    }

    let mut platform = WinitPlatform::init(&mut imgui);
    {
        let gl_window = display.gl_window();
        let window = gl_window.window();
        platform.attach_window(imgui.io_mut(), &window, HiDpiMode::Rounded);
    }

    let hidpi_factor = platform.hidpi_factor();
    let font_size = (13.0 * hidpi_factor) as f32;
    imgui.fonts().add_font(&[FontSource::DefaultFontData {
        config: Some(FontConfig {
            size_pixels: font_size,
            ..FontConfig::default()
        }),
    }]);

    imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

    let renderer = Renderer::init(&mut imgui, &display).expect("Failed to initialize renderer");

    System {
        event_loop,
        display,
        imgui,
        platform,
        renderer,
        font_size,
        clear_color: config.clear_color,
    }
}

impl System {
    pub fn main_loop<
        F: FnMut(&mut Ui, &Rc<backend::Context>, &mut Textures<Texture>) -> bool + 'static,
    >(
        self,
        mut run_ui: F,
    ) {
        let System {
            event_loop,
            display,
            mut imgui,
            mut platform,
            mut renderer,
            clear_color,
            ..
        } = self;
        let mut last_frame = Instant::now();

        event_loop.run(move |event, _, control_flow| match event {
            Event::NewEvents(_) => {
                let now = Instant::now();
                imgui.io_mut().update_delta_time(now - last_frame);
                last_frame = now;
            }
            Event::MainEventsCleared => {
                let gl_window = display.gl_window();
                platform
                    .prepare_frame(imgui.io_mut(), &gl_window.window())
                    .expect("Failed to prepare frame");
                gl_window.window().request_redraw();
            }
            Event::RedrawRequested(_) => {
                let mut ui = imgui.frame();

                let run = run_ui(&mut ui, display.get_context(), renderer.textures());
                if !run {
                    *control_flow = ControlFlow::Exit;
                }

                let gl_window = display.gl_window();
                let mut target = display.draw();
                target.clear_color(
                    clear_color[0],
                    clear_color[1],
                    clear_color[2],
                    clear_color[3],
                );
                platform.prepare_render(&ui, gl_window.window());
                let draw_data = ui.render();
                renderer
                    .render(&mut target, draw_data)
                    .expect("Rendering failed");
                target.finish().expect("Failed to swap buffers");
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            event => {
                let gl_window = display.gl_window();
                platform.handle_event(imgui.io_mut(), gl_window.window(), &event);
            }
        })
    }
}
