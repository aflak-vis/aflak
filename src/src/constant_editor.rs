use crate::primitives::{self, IOValue};
use imgui_file_explorer::{UiFileExplorer, CURRENT_FOLDER};
use imgui_tone_curve::UiToneCurve;
use node_editor::ConstantEditor;

use imgui::{ChildWindow, DrawListMut, Id, Ui};

#[derive(Default)]
pub struct MyConstantEditor;

impl ConstantEditor<primitives::IOValue> for MyConstantEditor {
    fn editor<'a, I>(
        &self,
        ui: &Ui,
        constant: &IOValue,
        id: I,
        read_only: bool,
        draw_list: &DrawListMut,
    ) -> Option<IOValue>
    where
        I: Into<Id<'a>>,
    {
        let id_stack = ui.push_id(id);

        let mut some_new_value = None;

        ui.group(|| some_new_value = inner_editor(ui, constant, read_only, &draw_list));

        if read_only && ui.is_item_hovered() {
            ui.tooltip_text("Read only, value is set by input!");
        }

        id_stack.pop();

        some_new_value
    }
}

fn inner_editor(
    ui: &Ui,
    constant: &IOValue,
    read_only: bool,
    draw_list: &DrawListMut,
) -> Option<IOValue> {
    match *constant {
        IOValue::Str(ref string) => {
            let mut out = String::with_capacity(1024);
            out.push_str(string);
            let changed = ui
                .input_text(format!("String value"), &mut out)
                .read_only(read_only)
                .build();
            if changed {
                Some(IOValue::Str(out.to_owned()))
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
                    .input_int(format!("Int value"), &mut out)
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
                .input_float(format!("Float value"), &mut f)
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
                .input_float2(format!("2 floats value"), &mut f2)
                .read_only(read_only)
                .build()
            {
                Some(IOValue::Float2(f2))
            } else {
                None
            }
        }
        IOValue::ToneCurve(ref state) => {
            let mut state_ret = None;
            ChildWindow::new("tonecurve_fileexplorer")
                .size([430.0, 430.0])
                .horizontal_scrollbar(true)
                .movable(false)
                .build(ui, || {
                    let result = ui.tone_curve(&mut state.clone(), &draw_list);
                    if let Ok(next_state) = result {
                        let next_state = next_state.unwrap();
                        if &next_state != state {
                            state_ret = Some(IOValue::ToneCurve(next_state));
                        }
                    }
                });
            state_ret
        }
        IOValue::Float3(ref floats) => {
            let mut f3 = *floats;
            if ui
                .input_float3(format!("3 floats value"), &mut f3)
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
            if ui.checkbox(format!("Bool value"), &mut b) {
                Some(IOValue::Bool(b))
            } else {
                None
            }
        }
        IOValue::Paths(ref file) => match file {
            primitives::PATHS::FileList(file) => {
                if read_only {
                    None
                } else {
                    let size = ui.item_rect_size();
                    let mut ret = Ok((None, None));
                    ChildWindow::new("path_fileexplorer")
                        .size([size[0].max(400.0), 150.0])
                        .horizontal_scrollbar(true)
                        .build(ui, || {
                            ret = ui.file_explorer(
                                CURRENT_FOLDER,
                                &[
                                    "fits", "fit", "fts", "cr2", "CR2", "RW2", "rw2", "nef", "NEF",
                                ],
                            );
                        });
                    ui.text(format!("Selected Files:"));
                    for single_file in file {
                        ui.text(single_file.to_str().unwrap_or("Unrepresentable path"));
                    }
                    if let Ok((Some(new_file), _)) = ret {
                        let mut already_exist = false;
                        let mut key = 0;
                        for single_file in file {
                            if *single_file == new_file {
                                already_exist = true;
                                break;
                            }
                            key += 1;
                        }
                        if already_exist {
                            let mut new_files = file.clone();
                            new_files.remove(key);
                            Some(IOValue::Paths(primitives::PATHS::FileList(
                                new_files.to_vec(),
                            )))
                        } else {
                            let mut new_files = file.clone();
                            new_files.push(new_file);
                            Some(IOValue::Paths(primitives::PATHS::FileList(
                                new_files.to_vec(),
                            )))
                        }
                    } else {
                        None
                    }
                }
            }
        },
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
