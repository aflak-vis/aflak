use std::collections::HashMap;
use std::error;

use glium;
use imgui::{ImGuiCond, ImMouseButton, ImString, Ui};

use aflak_plot::imshow::Textures;
use cake::OutputId;
use node_editor::NodeEditor;
use primitives::{IOErr, IOValue};

use constant_editor::MyConstantEditor;
use layout::LayoutEngine;
use output_window::OutputWindow;

pub type AflakNodeEditor = NodeEditor<'static, IOValue, IOErr, MyConstantEditor>;

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
        ui.window(im_str!("Node editor"))
            .position(
                self.layout_engine.default_editor_window_position(),
                ImGuiCond::FirstUseEver,
            )
            .size(
                self.layout_engine.default_editor_window_size(),
                ImGuiCond::FirstUseEver,
            )
            .build(|| {
                self.node_editor.render(ui);
            });
    }

    pub fn output_windows<F>(&mut self, ui: &Ui, gl_ctx: &F, textures: &mut Textures)
    where
        F: glium::backend::Facade,
    {
        let outputs = self.node_editor.outputs();
        for output in outputs {
            let output_window = self.output_windows.entry(output).or_default();
            output_window.draw(
                ui,
                output,
                &mut self.layout_engine,
                &mut self.node_editor,
                &mut self.error_alerts,
                gl_ctx,
                textures,
            );
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
