//! A minimal example that uses the node_editor crate
#[macro_use]
extern crate aflak_cake as cake;
extern crate aflak_imgui_glium_support as support;
#[macro_use]
extern crate imgui;
#[macro_use]
extern crate lazy_static;
extern crate node_editor;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate variant_name;
#[macro_use]
extern crate variant_name_derive;

use std::error::Error;
use std::fmt;

use imgui::{Id, ImString, Ui};
use variant_name::VariantName;

/// Values that we will transform in our node editor
/// Must implement VariantName, so that we can retrieve a variant type by name.
///
/// Our minimal editor will support integer and float!
#[derive(Clone, VariantName, Serialize, Deserialize)]
enum IOValue {
    Integer(i64),
    Float(f32),
}

/// Error type for failure during transformations
#[derive(Debug)]
struct IOErr;

impl fmt::Display for IOErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "IOErr")
    }
}

impl Error for IOErr {}

type MyNodeEditor = node_editor::NodeEditor<IOValue, IOErr>;

lazy_static! {
    /// Exhaustive list of all staticly loaded transforms that our node editor
    /// supports
    static ref TRANSFORMATIONS: Vec<cake::Transform<'static, IOValue, IOErr>> = {
        vec![
            cake_transform!(
                // description
                "A transformation that adds 1 to an integer",
                // version number
                1, 0, 0,
                // definition
                plus1<IOValue, IOErr>(i: Integer = 0) -> Integer {
                    vec![Ok(IOValue::Integer(i+1))]
                }
            ),
            cake_transform!(
                "Very slow transformation that returns two floats",
                1, 0, 0,
                slow<IOValue, IOErr>(i: Integer = 0) -> Float, Float {
                    use std::{thread, time};
                    // Do heavy work, i.e. sleep for 10 seconds
                    thread::sleep(time::Duration::from_secs(10));
                    vec![
                        // Success for first ouput
                        Ok(IOValue::Float(*i as f32)),
                        // Failure for second output
                        Err(IOErr),
                    ]
                }
            ),
        ]
    };
}

/// We need to define what kind of transformations can be applied to IOValue
/// Each transformation has a name by which they are retrieved
impl cake::NamedAlgorithms<IOErr> for IOValue {
    fn get_transform(s: &str) -> Option<&'static cake::Transform<'static, IOValue, IOErr>> {
        for t in TRANSFORMATIONS.iter() {
            if t.name() == s {
                return Some(t);
            }
        }
        None
    }
}

/// Define default values for each variant
impl cake::DefaultFor for IOValue {
    fn default_for(variant_name: &str) -> Self {
        match variant_name {
            "Integer" => IOValue::Integer(0),
            "Float" => IOValue::Float(0.0),
            _ => panic!("Unknown variant name provided: {}.", variant_name),
        }
    }
}

/// Define list of editable variants (those variants must implement an imgui editor)
impl cake::EditableVariants for IOValue {
    fn editable_variants() -> &'static [&'static str] {
        &["Integer", "Float"]
    }
}

fn integer_to_float(from: &IOValue) -> IOValue {
    if let IOValue::Integer(int) = from {
        IOValue::Float(*int as f32)
    } else {
        panic!("Unexpected input!")
    }
}
fn float_to_integer(from: &IOValue) -> IOValue {
    if let IOValue::Float(f) = from {
        IOValue::Integer(f.round() as _)
    } else {
        panic!("Unexpected input!")
    }
}

/// Define what type can be seamlessly casted into what
impl cake::ConvertibleVariants for IOValue {
    const CONVERTION_TABLE: &'static [cake::ConvertibleVariant<Self>] = &[
        cake::ConvertibleVariant {
            from: "Integer",
            into: "Float",
            f: integer_to_float,
        },
        cake::ConvertibleVariant {
            from: "Float",
            into: "Integer",
            f: float_to_integer,
        },
    ];
}

/// Define an editor to edit each constant type
#[derive(Default)]
struct MyConstantEditor;

impl node_editor::ConstantEditor<IOValue> for MyConstantEditor {
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
    }
}

fn main() {
    let transformations_ref: Vec<_> = TRANSFORMATIONS.iter().collect();

    let config = support::AppConfig {
        title: "Node editor example".to_owned(),
        ..Default::default()
    };
    let mut editor = MyNodeEditor::default();

    support::init(config).main_loop(move |ui, _, _| {
        let transformations = transformations_ref.as_slice();

        imgui::Window::new(im_str!("Node editor"))
            .size([900.0, 600.0], imgui::Condition::FirstUseEver)
            .build(ui, || {
                // Render main editor
                editor.render(ui, transformations, &MyConstantEditor);
            });

        // Render macro editors and popups
        editor.inner_editors_render(ui, transformations, &MyConstantEditor);
        editor.render_popups(ui);

        // Do something with outputs... For example, show them in a new imgui window
        let outputs = editor.outputs();
        for output in outputs {
            let window_name = ImString::new(format!("Output #{}", output.id()));
            imgui::Window::new(&window_name)
                .size([400.0, 400.0], imgui::Condition::FirstUseEver)
                .build(ui, || {
                    // Get result in each output
                    let compute_state = editor.compute_output(output);

                    // Show current state in a window
                    match compute_state {
                        None => {
                            ui.text("Initialiazing...");
                        }
                        Some(Err(e)) => {
                            ui.text(format!("Error... {:?}", e));
                        }
                        Some(Ok(result)) => {
                            let value = cake::compute::SuccessOut::take(result);
                            match &*value {
                                IOValue::Integer(integer) => {
                                    ui.text(format!("{}", integer));
                                }
                                IOValue::Float(float) => {
                                    ui.text(format!("{}", float));
                                }
                            }
                        }
                    }
                });
        }

        true
    })
}
