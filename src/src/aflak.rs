use std::collections::HashMap;
use std::error;

use glium;
use imgui::{ImGuiCond, ImMouseButton, ImString, Ui};

use aflak_plot::imshow::Textures;
use cake::OutputId;
use node_editor::NodeEditor;
use primitives::{IOErr, IOValue};

use constant_editor::MyConstantEditor;
use layout::{Layout, LayoutEngine};
use output_window::OutputWindow;

pub type AflakNodeEditor = NodeEditor<IOValue, IOErr, MyConstantEditor>;

pub struct Aflak {
    node_editor: AflakNodeEditor,
    layout_engine: LayoutEngine,
    output_windows: HashMap<OutputId, OutputWindow>,
    error_alerts: Vec<Box<error::Error>>,
}

impl Aflak {
    pub fn init(editor: AflakNodeEditor) -> Self {
        Self {
            node_editor: editor,
            layout_engine: LayoutEngine::new(),
            output_windows: HashMap::new(),
            error_alerts: vec![],
        }
    }

    pub fn node_editor(&mut self, ui: &Ui) {
        let display_size = ui.imgui().display_size();
        let Layout { position, size } = self.layout_engine.default_editor_layout(display_size);
        ui.window(im_str!("Node editor"))
            .position(position, ImGuiCond::FirstUseEver)
            .size(size, ImGuiCond::FirstUseEver)
            .build(|| {
                self.node_editor.render(ui);
            });
    }

    pub fn output_windows<F>(&mut self, ui: &Ui, gl_ctx: &F, textures: &mut Textures)
    where
        F: glium::backend::Facade,
    {
        let outputs = self.node_editor.outputs();
        let display_size = ui.imgui().display_size();
        for output in outputs {
            let output_window = self.output_windows.entry(output).or_default();
            let window_name = ImString::new(format!("Output #{}", output.id()));
            let mut window = ui.window(&window_name);
            if let Some(Layout { position, size }) = self
                .layout_engine
                .default_output_window_layout(&window_name, display_size)
            {
                window = window
                    .position(position, ImGuiCond::FirstUseEver)
                    .size(size, ImGuiCond::FirstUseEver);
            }
            let new_errors =
                output_window.draw(ui, output, window, &mut self.node_editor, gl_ctx, textures);
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
            if !ui.is_window_hovered() && ui.imgui().is_mouse_clicked(ImMouseButton::Left) {
                self.error_alerts.pop();
                ui.close_current_popup();
            }
        });
    }
}
