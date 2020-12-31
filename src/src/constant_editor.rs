use imgui_file_explorer::{UiFileExplorer, TOP_FOLDER};
use node_editor::ConstantEditor;
use primitives::{self, IOValue};

use imgui::{ChildWindow, Id, ImString, Ui};

#[derive(Default)]
pub struct MyConstantEditor;

impl ConstantEditor<primitives::IOValue> for MyConstantEditor {
    fn editor<'a, I>(&self, ui: &Ui, constant: &IOValue, id: I, read_only: bool) -> Option<IOValue>
    where
        I: Into<Id<'a>>,
    {
        let id_stack = ui.push_id(id);

        let mut some_new_value = None;

        ui.group(|| some_new_value = inner_editor(ui, constant, read_only));

        if read_only && ui.is_item_hovered() {
            ui.tooltip_text("Read only, value is set by input!");
        }

        id_stack.pop(ui);

        some_new_value
    }
}

fn inner_editor(ui: &Ui, constant: &IOValue, read_only: bool) -> Option<IOValue> {
    match *constant {
        IOValue::Str(ref string) => {
            let mut out = ImString::with_capacity(1024);
            out.push_str(string);
            let changed = ui
                .input_text(im_str!("String value"), &mut out)
                .read_only(read_only)
                .build();
            if changed {
                Some(IOValue::Str(out.to_str().to_owned()))
            } else {
                None
            }
        }
        IOValue::Integer(ref int) => {
            use std::i32;
            const MIN: i64 = i32::MIN as i64;
            const MAX: i64 = i32::MAX as i64;

            if MIN <= *int && *int <= MAX {
                let mut out = *int as i32;
                let changed = ui
                    .input_int(im_str!("Int value"), &mut out)
                    .read_only(read_only)
                    .build();
                if changed {
                    Some(IOValue::Integer(i64::from(out)))
                } else {
                    None
                }
            } else {
                ui.text(format!(
                    "Cannot edit integer smaller than {}\nor bigger than {}!\nGot {}.",
                    MIN, MAX, int
                ));
                None
            }
        }
        IOValue::Float(ref float) => {
            let mut f = *float;
            if ui
                .input_float(im_str!("Float value"), &mut f)
                .read_only(read_only)
                .build()
            {
                Some(IOValue::Float(f))
            } else {
                None
            }
        }
        IOValue::Float2(ref floats) => {
            let mut f2 = *floats;
            if ui
                .input_float2(im_str!("2 floats value"), &mut f2)
                .read_only(read_only)
                .build()
            {
                Some(IOValue::Float2(f2))
            } else {
                None
            }
        }
        IOValue::Float3(ref floats) => {
            let mut f3 = *floats;
            if ui
                .input_float3(im_str!("3 floats value"), &mut f3)
                .read_only(read_only)
                .build()
            {
                Some(IOValue::Float3(f3))
            } else {
                None
            }
        }
        IOValue::Bool(ref b) => {
            let mut b = *b;
            if ui.checkbox(im_str!("Bool value"), &mut b) {
                Some(IOValue::Bool(b))
            } else {
                None
            }
        }
        IOValue::Path(ref file) => {
            ui.text(file.to_str().unwrap_or("Unrepresentable path"));
            if read_only {
                None
            } else {
                let size = ui.item_rect_size();

                let mut ret = Ok(None);
                ChildWindow::new(im_str!("edit"))
                    .size([size[0].max(400.0), 150.0])
                    .horizontal_scrollbar(true)
                    .build(ui, || {
                        ret = ui.file_explorer(TOP_FOLDER, &["fits", "fit", "fts"]);
                    });
                if let Ok(Some(new_file)) = ret {
                    if *file != new_file {
                        Some(IOValue::Path(new_file))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
        IOValue::Roi(ref roi) => {
            match roi {
                primitives::ROI::All => ui.text("Whole image"),
                primitives::ROI::PixelList(_) => {
                    ui.text("Non-writable");
                    if ui.is_item_hovered() {
                        ui.tooltip(|| {
                            ui.text(" Please edit from output window ");
                        });
                    }
                }
            };
            None
        }
        _ => None,
    }
}
