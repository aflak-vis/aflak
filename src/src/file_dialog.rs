use std::error;
use std::path::PathBuf;

use imgui::{ChildWindow, ComboBox, ImString, Ui, Window};
use imgui_file_explorer::UiFileExplorer;

use aflak::AflakNodeEditor;
use templates;

pub struct FileDialog {
    title: ImString,
    selected_template: usize,
}

pub enum FileDialogEvent {
    Close,
    Selection(FileDialogResult),
}

pub struct FileDialogResult {
    path: PathBuf,
    template: String,
}

impl Default for FileDialog {
    fn default() -> Self {
        Self {
            title: ImString::new("Open file"),
            selected_template: 0,
        }
    }
}

impl FileDialog {
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
        Window::new(&self.title)
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
    pub fn into_node_editor(self) -> Result<AflakNodeEditor, impl error::Error> {
        if let Some(import_data) = templates::select_template(&self.template, self.path) {
            AflakNodeEditor::from_export_buf(import_data)
        } else {
            unreachable!("Got '{}', an unexpected result.", self.template)
        }
    }
}
