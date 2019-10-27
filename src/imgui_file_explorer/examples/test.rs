extern crate glium;
extern crate imgui;
extern crate imgui_file_explorer;
extern crate imgui_glium_renderer;
extern crate imgui_winit_support;
use imgui::*;
use imgui_file_explorer::UiFileExplorer;

mod support;

fn test(ui: &Ui) {
    let window = Window::new(im_str!("File Explorer"))
        .size([600.0, 400.0], Condition::Appearing)
        .position([200.0, 200.0], Condition::FirstUseEver);
    window.build(ui, || {
        ui.push_item_width(-140.0);
        let file = ui.file_explorer("/", &["fits", "csv"]);
        if let Ok(Some(file)) = file {
            println!("{:?}", file);
        }
    });
}

fn main() {
    support::init("imgui-file-explorer-test").main_loop(|_run, ui| {
        test(ui);
    });
}
