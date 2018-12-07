extern crate glium;

extern crate imgui;
extern crate imgui_glium_renderer;

mod glutin_support;

use std::error;
use std::fmt;
use std::rc::Rc;
use std::result;
use std::time::Instant;

use glium::{
    backend::{glutin::DisplayCreationError, Context, Facade},
    glutin, Display, Surface, SwapBuffersError, Texture2d,
};
use imgui::{ImGui, ImString, Textures, Ui};
use imgui_glium_renderer::{Renderer, RendererError};

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
    pub ini_filename: Option<ImString>,
    pub log_filename: Option<ImString>,
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
    Message(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Glutin(ref e) => e.fmt(f),
            Error::Render(ref e) => e.fmt(f),
            Error::SwapBuffers(ref e) => e.fmt(f),
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
    F: FnMut(&Ui, &Rc<Context>, &mut Textures<Texture2d>) -> bool,
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
    let window = display.gl_window();

    let mut imgui = ImGui::init();
    imgui.set_ini_filename(config.ini_filename);
    imgui.set_log_filename(config.log_filename);

    // We only use integer DPI factors, because the UI can get very blurry
    // otherwise.
    let hidpi_factor = window.get_hidpi_factor().round();

    let mut renderer = Renderer::init(&mut imgui, &display)?;

    glutin_support::configure_keys(&mut imgui);

    let mut last_frame = Instant::now();
    let mut quit = false;

    loop {
        events_loop.poll_events(|event| {
            use glium::glutin::{Event, WindowEvent::CloseRequested};

            glutin_support::handle_event(
                &mut imgui,
                &event,
                window.get_hidpi_factor(),
                hidpi_factor,
            );

            if let Event::WindowEvent { event, .. } = event {
                match event {
                    CloseRequested => quit = true,
                    _ => (),
                }
            }
        });

        let now = Instant::now();
        let delta = now - last_frame;
        let delta_s = delta.as_secs() as f32 + delta.subsec_nanos() as f32 / 1_000_000_000.0;
        last_frame = now;

        glutin_support::update_mouse_cursor(&imgui, &window);

        let frame_size = glutin_support::get_frame_size(&window, hidpi_factor).ok_or(
            Error::Message("Could not get frame size. Window no longer exists!"),
        )?;

        let ui = imgui.frame(frame_size, delta_s);
        if !run_ui(&ui, display.get_context(), renderer.textures()) {
            break;
        }

        let mut target = display.draw();
        target.clear_color(
            config.clear_color[0],
            config.clear_color[1],
            config.clear_color[2],
            config.clear_color[3],
        );
        renderer.render(&mut target, ui)?;
        target.finish()?;

        if quit {
            break;
        }
    }

    Ok(())
}
