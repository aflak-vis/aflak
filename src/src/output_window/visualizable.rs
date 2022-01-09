use std::fmt;

use imgui::{ImString, TreeNode, Ui, Window};

use crate::cake;
use crate::primitives::fitrs::Fits;

pub trait Visualizable {
    fn visualize(&self, ui: &Ui);

    fn draw<'ui>(&self, ui: &'ui Ui, window: Window<'_>) {
        window.build(ui, || self.visualize(ui));
    }
}

pub struct Initializing;

impl Visualizable for Initializing {
    fn visualize(&self, ui: &Ui) {
        ui.text("Initialiazing...");
    }
}

pub struct Unimplemented {
    variant: &'static str,
}

impl Unimplemented {
    pub fn new<T: cake::VariantName>(t: &T) -> Self {
        Unimplemented {
            variant: t.variant_name(),
        }
    }
}

impl Visualizable for Unimplemented {
    fn visualize(&self, ui: &Ui) {
        ui.text(format!(
            "Cannot visualize variable of type '{}'!",
            self.variant
        ));
    }
}

impl<E: fmt::Display> Visualizable for cake::compute::ComputeError<E> {
    fn visualize(&self, ui: &Ui) {
        ui.text_wrapped(&ImString::new(format!("{}", self)));
    }
}

impl Visualizable for Fits {
    fn visualize(&self, ui: &Ui) {
        let mut has_hdus = false;
        for (i, hdu) in self.iter().enumerate() {
            use crate::primitives::fitrs::HeaderValue::*;
            use std::borrow::Cow;

            has_hdus = true;

            let tree_name = match hdu.value("EXTNAME") {
                Some(CharacterString(extname)) => ImString::new(extname.as_str()),
                _ => {
                    if i == 0 {
                        im_str!("Primary HDU").to_owned()
                    } else {
                        ImString::new(format!("Hdu #{}", i))
                    }
                }
            };

            let id_stack = ui.push_id(i as i32);
            TreeNode::new(&tree_name).build(&ui, || {
                for (key, value) in &hdu {
                    ui.text(key);
                    if let Some(value) = value {
                        ui.same_line(/*150.0*/);
                        let value = match value {
                            CharacterString(s) => Cow::Borrowed(s.as_str()),
                            Logical(true) => Cow::Borrowed("True"),
                            Logical(false) => Cow::Borrowed("False"),
                            IntegerNumber(i) => Cow::Owned(format!("{}", i)),
                            RealFloatingNumber(f) => Cow::Owned(format!("{:E}", f)),
                            ComplexIntegerNumber(a, b) => Cow::Owned(format!("{} + {}i", a, b)),
                            ComplexFloatingNumber(a, b) => {
                                Cow::Owned(format!("{:E} + {:E}i", a, b))
                            }
                        };
                        ui.text(value);
                    }
                    ui.separator();
                }
            });
            id_stack.pop();
        }
        if !has_hdus {
            ui.text("Input Fits appears invalid. No HDU could be found.");
        }
    }
}
