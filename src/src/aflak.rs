use std::collections::HashMap;
use std::error;
use std::path::PathBuf;

use glium;
use imgui::{Condition, ImString, MenuItem, MouseButton, Ui, Window};

use crate::aflak_plot::{imshow::Textures, interactions::InteractionId};
use crate::cake::{NodeId, OutputId, Transform, TransformIdx};
use crate::primitives::{IOErr, IOValue};
use node_editor::NodeEditor;

use crate::constant_editor::MyConstantEditor;
use crate::file_dialog::{FileDialog, FileDialogEvent};
use crate::implot::Context;
use crate::layout::{Layout, LayoutEngine};
use crate::output_window::OutputWindow;
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
    pub show_bind_manager: bool,
    copying: Option<(InteractionId, TransformIdx)>,
    attaching: Option<(OutputId, TransformIdx, usize)>,
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
            show_bind_manager: false,
            copying: None,
            attaching: None,
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
                if MenuItem::new(im_str!("Quit"))
                    .shortcut(im_str!("Alt+F4"))
                    .build(ui)
                {
                    self.quit = true;
                }
                menu.end(ui);
            }
            if let Some(menu) = ui.begin_menu(im_str!("Others"), true) {
                if MenuItem::new(im_str!("Metrics")).build(ui) {
                    self.show_metrics = !self.show_metrics;
                }
                if MenuItem::new(im_str!("Bind manager")).build(ui) {
                    self.show_bind_manager = !self.show_bind_manager;
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
                    .render(ui, addable_nodes, &MyConstantEditor, &mut self.attaching);
            });
        self.node_editor.inner_editors_render(
            ui,
            addable_nodes,
            &MyConstantEditor,
            &mut self.attaching,
        );
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
            let output_id = output.0;
            let output_name = output.1;
            let output_window = self.output_windows.entry(output_id).or_default();
            let window_name = ImString::new(format!("{}###{:?}", output_name, output_id));
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
                &mut self.copying,
                &mut self.attaching,
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

    pub fn bind_manager(&mut self, ui: &Ui) {
        Window::new(im_str!("Bind Manager")).build(ui, || {
            ui.text("Bindings:");
            let outputs = self.node_editor.outputs();
            let dst = &self.node_editor.dst;
            let mut bind_count = 0;
            for output in outputs {
                let output_window = self.output_windows.entry(output).or_default();
                let editable_values = output_window.editable_values.clone();
                let mut remove_id = None;
                for (interaction_id, transformidx) in editable_values.iter() {
                    if let Some(macro_id) = transformidx.macro_id() {
                        if let Some(macr) = &self.node_editor.macros.get_macro(macro_id) {
                            for (nodeid, node) in macr.read().dst().nodes_iter() {
                                if let NodeId::Transform(t_idx) = nodeid {
                                    if t_idx == *transformidx {
                                        ui.text(im_str!(
                                            "{:?} <--> {:?} in Macro {:?}",
                                            output,
                                            node.name(&nodeid),
                                            macr.name(),
                                        ));
                                        let p = ui.cursor_screen_pos();
                                        ui.set_cursor_screen_pos([p[0] + 150.0, p[1]]);
                                        bind_count += 1;
                                        if ui.button(
                                            &ImString::new(format!("Remove bind {}", bind_count)),
                                            [0.0, 0.0],
                                        ) {
                                            remove_id = Some(interaction_id);
                                        }
                                        if ui.is_item_hovered() {
                                            ui.tooltip_text(im_str!(
                                                "Remove this bind and create new bind"
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        for (nodeid, node) in dst.nodes_iter() {
                            if let NodeId::Transform(t_idx) = nodeid {
                                if t_idx == *transformidx {
                                    ui.text(im_str!("{:?} <--> {:?}", output, node.name(&nodeid)));
                                    let p = ui.cursor_screen_pos();
                                    ui.set_cursor_screen_pos([p[0] + 150.0, p[1]]);
                                    bind_count += 1;
                                    if ui.button(
                                        &ImString::new(format!("Remove bind {}", bind_count)),
                                        [0.0, 0.0],
                                    ) {
                                        remove_id = Some(interaction_id);
                                    }
                                    if ui.is_item_hovered() {
                                        ui.tooltip_text(im_str!(
                                            "Remove this bind and create new bind"
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(remove_id) = remove_id {
                    output_window.editable_values.remove(remove_id);
                }
            }
        });
    }
}
