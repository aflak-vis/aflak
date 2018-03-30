extern crate glium;
#[macro_use]
extern crate imgui;
extern crate imgui_glium_renderer;

mod support;

extern crate aflak_cake as cake;
extern crate aflak_primitives as primitives;
extern crate node_editor;
use node_editor::NodeEditor;

const CLEAR_COLOR: [f32; 4] = [0.05, 0.05, 0.05, 1.0];

fn main() {
    let transformations_ref = primitives::TRANSFORMATIONS.iter().collect::<Vec<_>>();
    let transformations = transformations_ref.as_slice();
    let mut dst = cake::DST::new();
    let a = dst.add_transform(transformations[0]);
    let _b = dst.add_transform(transformations[0]);
    let c = dst.add_transform(transformations[1]);
    dst.connect(cake::Output::new(a, 0), cake::Input::new(c, 0))
        .unwrap();
    let mut node_editor = NodeEditor::from_dst(dst, transformations);
    support::run("Node editor example".to_owned(), CLEAR_COLOR, |ui| {
        ui.window(im_str!("Node editor")).build(|| {
            node_editor.render(ui);
        });
        true
    });
}
