extern crate aflak_cake as cake;
#[macro_use]
extern crate imgui;
extern crate imgui_sys as sys;

use cake::{TransformIdx, Transformation, DST};
use imgui::{ImGuiCol, ImGuiMouseCursor, ImString, Ui};

pub struct NodeEditor<'t, T: 't + Clone, E: 't> {
    dst: DST<'t, T, E>,
    addable_nodes: &'t [&'t Transformation<T, E>],
    active_node: Option<TransformIdx>,
    pub show_left_pane: bool,
    left_pane_size: Option<f32>,
}

impl<'t, T: Clone, E> NodeEditor<'t, T, E> {
    pub fn new(addable_nodes: &'t [&'t Transformation<T, E>]) -> Self {
        Self {
            dst: DST::new(),
            addable_nodes,
            active_node: None,
            show_left_pane: true,
            left_pane_size: None,
        }
    }

    pub fn with_left_pane(mut self, show_left_pane: bool) -> Self {
        self.show_left_pane = show_left_pane;
        self
    }

    pub fn render(&mut self, ui: &Ui) {
        if self.show_left_pane {
            self.render_left_pane(ui);
        }
    }

    fn render_left_pane(&mut self, ui: &Ui) {
        const LEFT_PANE_DEFAULT_RELATIVE_WIDTH: f32 = 0.2;
        let window_size = ui.imgui().get_window_size();
        let pane_width = *self.left_pane_size
            .get_or_insert_with(|| window_size.0 * LEFT_PANE_DEFAULT_RELATIVE_WIDTH);

        ui.child_frame(im_str!("node_list"), (pane_width, 0.0))
            .build(|| {
                ui.spacing();
                ui.separator();
                if ui.collapsing_header(im_str!("Node List##node_list_1"))
                    .build()
                {
                    ui.separator();
                    self.show_node_list(ui);
                }
                ui.separator();
                self.active_node.map(|node| {
                    ui.spacing();
                    ui.separator();
                    if ui.collapsing_header(im_str!("Active Node##activeNode"))
                        .build()
                    {
                        ui.separator();
                        // TODO: Show active node info
                        ui.text(ImString::new(format!("Selected node's ID: {:?}", node)));
                    }
                    ui.separator();
                });
            });

        // Horizontal splitter
        ui.same_line(0.0);
        const SPLITTER_WIDTH: f32 = 6.0;
        const SPLITTER_DESIGN: [(ImGuiCol, (f32, f32, f32, f32)); 3] = [
            (ImGuiCol::Button, (1.0, 1.0, 1.0, 0.2)),
            (ImGuiCol::ButtonHovered, (1.0, 1.0, 1.0, 0.35)),
            (ImGuiCol::ButtonActive, (1.0, 1.0, 1.0, 0.5)),
        ];
        ui.with_color_vars(&SPLITTER_DESIGN, || {
            ui.button(im_str!("##hsplitter1"), (SPLITTER_WIDTH, -1.0));
            let splitter_active = ui.is_item_active();
            if ui.is_item_hovered() || splitter_active {
                ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeEW);
            }
            if let Some(ref mut w) = self.left_pane_size {
                if splitter_active {
                    let mouse_delta_x = ui.imgui().mouse_delta().0;
                    *w += mouse_delta_x;
                }
                let style = ui.imgui().style();
                let minw = style.window_padding.x + style.frame_padding.x;
                let maxw = minw + window_size.0 - SPLITTER_WIDTH - style.window_min_size.x;
                if *w > maxw {
                    *w = maxw;
                } else if *w < minw {
                    *w = minw;
                }
            }
        });
        ui.same_line(0.0);
    }

    fn show_node_list(&self, ui: &Ui) {
        // TODO
        ui.text(im_str!("TODO SHOW NODE LIST"));
    }
}
