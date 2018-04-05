extern crate glium;
#[macro_use]
extern crate imgui;
extern crate imgui_glium_renderer;

mod support;

extern crate aflak_cake as cake;
extern crate aflak_primitives as primitives;
extern crate node_editor;
use node_editor::NodeEditor;

use imgui::ImString;

const CLEAR_COLOR: [f32; 4] = [0.05, 0.05, 0.05, 1.0];

fn main() {
    let transformations_ref = primitives::TRANSFORMATIONS.iter().collect::<Vec<_>>();
    let transformations = transformations_ref.as_slice();
    let string_constant = cake::Transformation::new_constant(primitives::IOValue::Str("TEST".to_owned()));
    let mut dst = cake::DST::new();
    let a = dst.add_transform(transformations[0]);
    let _b = dst.add_transform(transformations[0]);
    let c = dst.add_transform(transformations[1]);
    let _d = dst.add_transform(&string_constant);
    dst.connect(cake::Output::new(a, 0), cake::Input::new(c, 0))
        .unwrap();
    dst.attach_output(cake::Output::new(c, 0)).unwrap();
    let mut node_editor = NodeEditor::from_dst(dst, transformations);
    support::run("Node editor example".to_owned(), CLEAR_COLOR, |ui| {
        ui.window(im_str!("Node editor")).build(|| {
            node_editor.render(ui);
        });
        let outputs = node_editor.outputs();
        for output in outputs {
            let window_name = ImString::new(format!("{:?}", output));
            ui.window(&window_name).build(|| {
                let result = node_editor.compute_output(&output);
                match result {
                    Err(e) => {
                        ui.text(format!("{:?}", e));
                    },
                    Ok(result) => {
                        match result {
                            primitives::IOValue::Str(string) => {
                                ui.text(format!("{:?}", string));
                            },
                            _ => {
                                ui.text("Unimplemented");
                            }
                        }
                    }
                }
            });
        }
        true
    });
}
