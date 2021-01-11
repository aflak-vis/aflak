use std::collections::HashMap;
use std::error;
use std::path::PathBuf;

use glium;
use imgui::{Condition, ImString, MenuItem, MouseButton, Ui, Window};

use aflak_plot::imshow::Textures;
use cake::{OutputId, Transform};
use node_editor::NodeEditor;
use primitives::{IOErr, IOValue};

use constant_editor::MyConstantEditor;
use file_dialog::{FileDialog, FileDialogEvent};
use implot::Context;
use layout::{Layout, LayoutEngine};
use output_window::OutputWindow;

pub type AflakNodeEditor = NodeEditor<IOValue, IOErr>;

pub struct Aflak {
    node_editor: AflakNodeEditor,
    layout_engine: LayoutEngine,
    output_windows: HashMap<OutputId, OutputWindow>,
    error_alerts: Vec<Box<dyn error::Error>>,
    pub quit: bool,
    file_dialog: Option<FileDialog>,
    recent_files: Vec<PathBuf>,
    pub show_metrics: bool,
}

impl Aflak {
    pub fn init(editor: AflakNodeEditor) -> Self {
        Self {
            node_editor: editor,
            layout_engine: LayoutEngine::new(),
            output_windows: HashMap::new(),
            error_alerts: vec![],
            quit: false,
            file_dialog: None,
            recent_files: vec![],
            show_metrics: false,
        }
    }

    pub fn main_menu_bar(&mut self, ui: &Ui) {
        let mut new_editor = false;

        if let Some(menu_bar) = ui.begin_main_menu_bar() {
            if let Some(menu) = ui.begin_menu(im_str!("File"), true) {
                if MenuItem::new(im_str!("New")).build(ui) {
                    new_editor = true;
                }
                if MenuItem::new(im_str!("Open FITS")).build(ui) {
                    self.file_dialog = Some(FileDialog::default());
                }
                if let Some(menu) =
                    ui.begin_menu(im_str!("Open Recent"), !self.recent_files.is_empty())
                {
                    for file in self.recent_files.iter().rev() {
                        if MenuItem::new(&ImString::new(file.to_string_lossy())).build(ui) {
                            self.file_dialog = Some(FileDialog::with_path(file.clone()));
                        }
                    }
                    menu.end(ui);
                }
                ui.separator();
                if MenuItem::new(im_str!("Metrics")).build(ui) {
                    self.show_metrics = !self.show_metrics;
                }
                if MenuItem::new(im_str!("Quit"))
                    .shortcut(im_str!("Alt+F4"))
                    .build(ui)
                {
                    self.quit = true;
                }
                menu.end(ui);
            }
            menu_bar.end(ui);
        }

        if new_editor {
            ui.open_popup(im_str!("Start new node program"));
        }
        ui.popup_modal(im_str!("Start new node program"))
            .always_auto_resize(true)
            .build(|| {
                ui.text("The current node program will be lost. Proceed?");
                ui.separator();
                if ui.button(im_str!("OK"), [120.0, 0.0]) {
                    self.node_editor = NodeEditor::default();
                    ui.close_current_popup();
                }
                ui.same_line(0.0);
                if ui.button(im_str!("Cancel"), [120.0, 0.0]) {
                    ui.close_current_popup();
                }
            });
    }

    pub fn node_editor(&mut self, ui: &Ui, addable_nodes: &[&'static Transform<IOValue, IOErr>]) {
        let display_size = ui.io().display_size;
        let Layout { position, size } = self.layout_engine.default_editor_layout(display_size);
        Window::new(im_str!("Node editor"))
            .position(position, Condition::FirstUseEver)
            .size(size, Condition::FirstUseEver)
            .build(ui, || {
                self.node_editor
                    .render(ui, addable_nodes, &MyConstantEditor);
            });
        self.node_editor
            .inner_editors_render(ui, addable_nodes, &MyConstantEditor);
        self.node_editor.render_popups(ui);
    }

    pub fn output_windows<F>(
        &mut self,
        ui: &Ui,
        gl_ctx: &F,
        textures: &mut Textures,
        plotcontext: &Context,
    ) where
        F: glium::backend::Facade,
    {
        let outputs = self.node_editor.outputs();
        let display_size = ui.io().display_size;
        for output in outputs {
            let output_window = self.output_windows.entry(output).or_default();
            let window_name = ImString::new(format!("Output #{}", output.id()));
            let mut window = Window::new(&window_name);
            if let Some(Layout { position, size }) = self
                .layout_engine
                .default_output_window_layout(&window_name, display_size)
            {
                window = window
                    .position(position, Condition::FirstUseEver)
                    .size(size, Condition::FirstUseEver);
            }
            let new_errors = output_window.draw(
                ui,
                output,
                window,
                &mut self.node_editor,
                gl_ctx,
                textures,
                &plotcontext,
            );
            self.error_alerts.extend(new_errors);
        }
    }

    pub fn show_errors(&mut self, ui: &Ui) {
        if !self.error_alerts.is_empty() {
            ui.open_popup(im_str!("Error"));
        }
        ui.popup_modal(im_str!("Error")).build(|| {
            {
                let e = &self.error_alerts[self.error_alerts.len() - 1];
                ui.text(&ImString::new(format!("{}", e)));
            }
            if !ui.is_window_hovered() && ui.is_mouse_clicked(MouseButton::Left) {
                self.error_alerts.pop();
                ui.close_current_popup();
            }
        });
    }

    pub fn file_dialog(&mut self, ui: &Ui) {
        if let Some(dialog) = &mut self.file_dialog {
            match dialog.build(ui) {
                Some(FileDialogEvent::Selection(result)) => {
                    match result.to_node_editor() {
                        Ok(node_editor) => self.node_editor = node_editor,
                        Err(e) => self.error_alerts.push(Box::new(e)),
                    }
                    if !self.recent_files.contains(&result.path) {
                        self.recent_files.push(result.path);
                    }
                    self.file_dialog = None;
                }
                Some(FileDialogEvent::Close) => self.file_dialog = None,
                None => {}
            }
        }
    }
}
