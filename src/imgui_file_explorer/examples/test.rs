extern crate glium;
extern crate imgui;
extern crate imgui_file_explorer;
extern crate imgui_glium_renderer;
use imgui::*;
use imgui_file_explorer::UiFileExplorer;

mod support;

const CLEAR_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

fn test(ui: &Ui) {
    let window = ui
        .window(im_str!("File Explorer"))
        .size((600.0, 400.0), ImGuiCond::Appearing)
        .position((200.0, 200.0), ImGuiCond::FirstUseEver);
    window.build(|| {
        ui.push_item_width(-140.0);
        let file = ui.file_explorer("/", &[".fits", ".csv"]);
        if let Ok(Some(file)) = file {
            println!("{:?}", file);
        }
    });
}

fn main() {
    support::run("imgui-file-explorer-test".to_owned(), CLEAR_COLOR, |ui| {
        test(ui);
        true
    });
}
