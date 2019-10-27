extern crate clipboard;
extern crate glium;

extern crate imgui;
extern crate imgui_glium_renderer;
extern crate imgui_winit_support;

mod clipboard_support;

use std::error;
use std::fmt;
use std::path::PathBuf;
use std::rc::Rc;
use std::result;
use std::time::Instant;

use glium::{
    backend::{self, glutin::DisplayCreationError, Facade},
    glutin::{self, Event, WindowEvent},
    Display, Surface, SwapBuffersError, Texture2d,
};
use imgui::{Context, FontConfig, FontSource, Textures, Ui};
use imgui_glium_renderer::{Renderer, RendererError};
use imgui_winit_support::{HiDpiMode, WinitPlatform};

#[derive(Copy, Clone, PartialEq, Debug, Default)]
struct MouseState {
    pos: (i32, i32),
    pressed: (bool, bool, bool),
    wheel: f32,
}

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

#[derive(Debug)]
pub enum Error {
    Glutin(DisplayCreationError),
    Render(RendererError),
    SwapBuffers(SwapBuffersError),
    PrepareFrame,
    Message(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Glutin(ref e) => e.fmt(f),
            Error::Render(ref e) => e.fmt(f),
            Error::SwapBuffers(ref e) => e.fmt(f),
            Error::PrepareFrame => write!(f, "Failed to prepare frame"),
            Error::Message(msg) => write!(f, "{}", msg),
        }
    }
}

impl error::Error for Error {}

pub type Result<T> = result::Result<T, Error>;

impl From<DisplayCreationError> for Error {
    fn from(e: DisplayCreationError) -> Self {
        Error::Glutin(e)
    }
}

impl From<RendererError> for Error {
    fn from(e: RendererError) -> Self {
        Error::Render(e)
    }
}

impl From<SwapBuffersError> for Error {
    fn from(e: SwapBuffersError) -> Self {
        Error::SwapBuffers(e)
    }
}

pub fn run<F>(config: AppConfig, mut run_ui: F) -> Result<()>
where
    F: FnMut(&mut Ui, &Rc<backend::Context>, &mut Textures<Rc<Texture2d>>) -> bool,
{
    let mut events_loop = glutin::EventsLoop::new();
    let context = glutin::ContextBuilder::new().with_vsync(true);
    let builder = glutin::WindowBuilder::new()
        .with_title(config.title)
        .with_dimensions(glutin::dpi::LogicalSize::new(
            config.window_width as f64,
            config.window_height as f64,
        ))
        .with_maximized(config.maximized);
    let display = Display::new(builder, context, &events_loop)?;
    let gl_window = display.gl_window();
    let window = gl_window.window();

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

    let font_size = 13.0;
    imgui.fonts().add_font(&[FontSource::DefaultFontData {
        config: Some(FontConfig {
            size_pixels: font_size,
            ..FontConfig::default()
        }),
    }]);

    imgui.io_mut().font_global_scale = 1.0;

    let mut renderer = Renderer::init(&mut imgui, &display)?;

    let mut last_frame = Instant::now();
    let mut quit = false;

    loop {
        events_loop.poll_events(|event| {
            platform.handle_event(imgui.io_mut(), &window, &event);

            if let Event::WindowEvent { event, .. } = event {
                if let WindowEvent::CloseRequested = event {
                    quit = true;
                }
            }
        });

        let io = imgui.io_mut();
        platform
            .prepare_frame(io, &window)
            .map_err(|_| Error::PrepareFrame)?;
        last_frame = io.update_delta_time(last_frame);
        let mut ui = imgui.frame();
        if !run_ui(&mut ui, display.get_context(), renderer.textures()) {
            break;
        }

        let mut target = display.draw();
        target.clear_color(
            config.clear_color[0],
            config.clear_color[1],
            config.clear_color[2],
            config.clear_color[3],
        );
        platform.prepare_render(&ui, &window);
        let draw_data = ui.render();
        renderer.render(&mut target, draw_data)?;
        target.finish()?;

        if quit {
            break;
        }
    }

    Ok(())
}
