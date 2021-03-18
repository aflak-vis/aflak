extern crate aflak_imgui_glium_support as support;
extern crate imgui;
extern crate imgui_file_explorer;
use imgui::*;
use imgui_file_explorer::UiFileExplorer;

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
    let config = support::AppConfig {
        title: "Example file explorer".to_owned(),
        ..Default::default()
    };

    support::init(config).main_loop(|ui, _, _| {
        test(ui);
        true
    });
}
