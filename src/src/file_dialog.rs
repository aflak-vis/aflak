use std::error;
use std::path::PathBuf;

use imgui::{ChildWindow, ComboBox, Condition, ImString, Ui, Window};
use imgui_file_explorer::UiFileExplorer;

use crate::aflak::AflakNodeEditor;
use crate::templates;

pub struct FileDialog {
    selected_template: usize,
    file_selection: FileSelection,
}

enum FileSelection {
    FileNotSelected,
    FileSelected { path: PathBuf },
}

pub enum FileDialogEvent {
    Close,
    Selection(FileDialogResult),
}

pub struct FileDialogResult {
    pub path: PathBuf,
    template: String,
}

impl Default for FileDialog {
    fn default() -> Self {
        Self {
            selected_template: 0,
            file_selection: FileSelection::FileNotSelected,
        }
    }
}

impl FileDialog {
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            selected_template: 0,
            file_selection: FileSelection::FileSelected { path },
        }
    }

    pub fn build(&mut self, ui: &Ui) -> Option<FileDialogEvent> {
        let selected_template = &mut self.selected_template;
        let mut some_path = None;
        let mut opened = true;
        let template_names = [
            im_str!("waveform"),
            im_str!("equivalent_width"),
            im_str!("fits_cleaning"),
            im_str!("velocity_field"),
        ];
        match &self.file_selection {
            FileSelection::FileNotSelected => {
                Window::new(im_str!("Open file"))
                    .focus_on_appearing(true)
                    .opened(&mut opened)
                    .build(ui, || {
                        ComboBox::new(im_str!("Template")).build_simple_string(
                            ui,
                            selected_template,
                            &template_names,
                        );
                        ChildWindow::new(im_str!("file-explorer"))
                            .size([0.0, 512.0])
                            .build(ui, || {
                                if let Ok(path) = ui.file_explorer("/", &["fits"]) {
                                    some_path = path;
                                }
                            })
                    });
            }
            FileSelection::FileSelected { path } => {
                Window::new(&ImString::new(format!("Open {:?}", path)))
                    .focus_on_appearing(true)
                    .opened(&mut opened)
                    .size([512.0, 0.0], Condition::FirstUseEver)
                    .build(ui, || {
                        ComboBox::new(im_str!("Template")).build_simple_string(
                            ui,
                            selected_template,
                            &template_names,
                        );
                        if ui.button(im_str!("OK"), [0.0, 0.0]) {
                            some_path = Some(path.clone());
                        }
                    });
            }
        }
        if !opened {
            Some(FileDialogEvent::Close)
        } else if let Some(path) = some_path {
            Some(FileDialogEvent::Selection(FileDialogResult {
                path,
                template: format!("{}", template_names[*selected_template]),
            }))
        } else {
            None
        }
    }
}

impl FileDialogResult {
    pub fn to_node_editor(&self) -> Result<AflakNodeEditor, impl error::Error> {
        if let Some(import_data) = templates::select_template(&self.template, &self.path) {
            AflakNodeEditor::from_export_buf(import_data)
        } else {
            unreachable!("Got '{}', an unexpected result.", self.template)
        }
    }
}
