#[macro_use]
extern crate aflak_cake as cake;
extern crate aflak_primitives as primitives;
extern crate visprog_draw as vis_prog;

extern crate glium;
use cake::{Input, NamedAlgorithms, Output, DST};
use primitives::IOValue;

fn main() {
    let draw_options = vis_prog::DrawOptions::default();
    let path_string = cake_constant!(path, IOValue::Str("/path/to/file".to_owned()));
    let mut dst = DST::new();
    let a = dst.add_transform(&path_string);
    let b = dst.add_transform(IOValue::get_transform("open_fits").expect("Find open_fits"));
    let _out = dst.attach_output(Output::new(b, 0)).unwrap();
    dst.connect(Output::new(a, 0), Input::new(b, 0)).unwrap();

    let diagram = vis_prog::Diagram::new(&dst);

    let mut events_loop = glium::glutin::EventsLoop::new();
    let window = glium::glutin::WindowBuilder::new()
        .with_dimensions(1024, 768)
        .with_title("Aflak DRAW DST");
    let context = glium::glutin::ContextBuilder::new();
    let display = glium::Display::new(window, context, &events_loop).unwrap();
    let mut quit = false;
    loop {
        events_loop.poll_events(|ev| match ev {
            glium::glutin::Event::WindowEvent { event, .. } => match event {
                glium::glutin::WindowEvent::Closed => quit = true,
                _ => (),
            },
            _ => (),
        });
        if quit {
            break;
        }

        let mut target = display.draw();
        vis_prog::draw(&mut target, &diagram, &draw_options).unwrap();
        target.finish().unwrap();
    }
}
