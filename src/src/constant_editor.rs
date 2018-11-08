use imgui_file_explorer::{UiFileExplorer, TOP_FOLDER};
use node_editor::ConstantEditor;
use primitives;

use imgui::{ImString, Ui};

#[derive(Default)]
pub struct MyConstantEditor;

impl ConstantEditor<primitives::IOValue> for MyConstantEditor {
    fn editor(&self, ui: &Ui, constant: &mut primitives::IOValue) -> bool {
        use primitives::IOValue;

        ui.push_id(constant as *const primitives::IOValue as i32);
        let changed = match *constant {
            IOValue::Str(ref mut string) => {
                let mut out = ImString::with_capacity(1024);
                out.push_str(string);
                let changed = ui.input_text(im_str!("String value"), &mut out).build();
                *string = out.to_str().to_owned();
                changed
            }
            IOValue::Integer(ref mut int) => {
                let mut out = *int as i32;
                let changed = ui.input_int(im_str!("Int value"), &mut out).build();
                *int = out as i64;
                changed
            }
            IOValue::Float(ref mut float) => ui.input_float(im_str!("Float value"), float).build(),
            IOValue::Float2(ref mut floats) => {
                ui.input_float2(im_str!("2 floats value"), floats).build()
            }
            IOValue::Float3(ref mut floats) => {
                ui.input_float3(im_str!("3 floats value"), floats).build()
            }
            IOValue::Path(ref mut file) => {
                ui.text(file.to_str().unwrap_or("Unrepresentable path"));
                let size = ui.get_item_rect_size();

                let mut ret = Ok(None);
                ui.child_frame(im_str!("edit"), (size.0.max(200.0), 150.0))
                    .scrollbar_horizontal(true)
                    .build(|| {
                        ret = ui.file_explorer(TOP_FOLDER, &["fits"]);
                    });
                if let Ok(Some(new_file)) = ret {
                    if *file != new_file {
                        *file = new_file;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => false,
        };
        ui.pop_id();

        changed
    }
}
