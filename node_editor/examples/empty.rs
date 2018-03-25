extern crate glium;
#[macro_use]
extern crate imgui;
extern crate imgui_glium_renderer;

mod support;

extern crate aflak_primitives as primitives;
extern crate node_editor;
use node_editor::NodeEditor;

const CLEAR_COLOR: [f32; 4] = [0.05, 0.05, 0.05, 1.0];

fn main() {
    let transformations_ref = primitives::TRANSFORMATIONS.iter().collect::<Vec<_>>();
    let mut node_editor = NodeEditor::new(transformations_ref.as_slice());
    support::run("Node editor example".to_owned(), CLEAR_COLOR, |ui| {
        ui.window(im_str!("Node editor")).build(|| {
            node_editor.render(ui);
        });
        true
    });
}
