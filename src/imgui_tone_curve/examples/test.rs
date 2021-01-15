extern crate glium;
extern crate imgui;
extern crate imgui_glium_renderer;
extern crate imgui_tone_curve;
extern crate imgui_winit_support;
use imgui::*;
use imgui_tone_curve::{ToneCurveState, UiToneCurve};

mod support;

fn test(ui: &Ui, mut state: &mut ToneCurveState, draw_list: WindowDrawList) {
    let window = Window::new(im_str!("Tone Curve"))
        .size([600.0, 400.0], Condition::Appearing)
        .position([200.0, 200.0], Condition::FirstUseEver);
    window.build(ui, || {
        let vectors = ui.tone_curve(&mut state, &draw_list);
        if let Ok(vectors) = vectors {
            let vectors = vectors.unwrap();
            ui.text(format!("control points: {:?}", vectors.control_points()));
            ui.text_wrapped(&im_str!("data array: {:?}", vectors.array()));
        }
    });
}

fn main() {
    let mut state = ToneCurveState::default();
    support::init("imgui-tone_curve-test").main_loop(move |_run, ui, _, _| {
        let draw_list = ui.get_window_draw_list();
        test(ui, &mut state, draw_list);
        true
    });
}
