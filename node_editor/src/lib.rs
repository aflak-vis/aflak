extern crate aflak_cake as cake;
#[macro_use]
extern crate imgui;
extern crate imgui_sys as sys;

use std::collections::BTreeMap;
use cake::{TransformIdx, Transformation, DST};
use imgui::{ImGuiCol, ImGuiMouseCursor, ImGuiSelectableFlags, ImMouseButton, ImStr, ImString,
            ImVec2, StyleVar, Ui};

pub struct NodeEditor<'t, T: 't + Clone, E: 't> {
    dst: DST<'t, T, E>,
    addable_nodes: &'t [&'t Transformation<T, E>],
    node_states: BTreeMap<TransformIdx, NodeState>,
    active_node: Option<TransformIdx>,
    pub show_left_pane: bool,
    left_pane_size: Option<f32>,
    pub show_top_pane: bool,
    pub show_connection_names: bool,
    scrolling: (f32, f32),
    pub show_grid: bool,
}

impl<'t, T: Clone, E> Default for NodeEditor<'t, T, E> {
    fn default() -> Self {
        Self {
            dst: DST::new(),
            addable_nodes: &[],
            node_states: BTreeMap::new(),
            active_node: None,
            show_left_pane: true,
            left_pane_size: None,
            show_top_pane: true,
            show_connection_names: true,
            scrolling: (0.0, 0.0),
            show_grid: true,
        }
    }
}

struct NodeState {
    selected: bool,
}

impl Default for NodeState {
    fn default() -> Self {
        Self { selected: false }
    }
}

impl<'t, T: Clone, E> NodeEditor<'t, T, E> {
    pub fn new(addable_nodes: &'t [&'t Transformation<T, E>]) -> Self {
        Self {
            addable_nodes,
            ..Default::default()
        }
    }

    pub fn from_dst(dst: DST<'t, T, E>, addable_nodes: &'t [&'t Transformation<T, E>]) -> Self {
        Self {
            dst,
            addable_nodes,
            ..Default::default()
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
        self.render_graph_node(ui);
    }

    fn render_left_pane(&mut self, ui: &Ui) {
        const LEFT_PANE_DEFAULT_RELATIVE_WIDTH: f32 = 0.2;
        let window_size = ui.get_window_size();
        let pane_width = *self.left_pane_size
            .get_or_insert_with(|| window_size.0 * LEFT_PANE_DEFAULT_RELATIVE_WIDTH);

        ui.child_frame(im_str!("node_list"), (pane_width, 0.0))
            .build(|| {
                ui.spacing();
                ui.separator();
                if ui.collapsing_header(im_str!("Node List##node_list_1"))
                    .default_open(true)
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
                        .default_open(true)
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

    fn show_node_list(&mut self, ui: &Ui) {
        for (idx, node) in self.dst.transforms_iter() {
            ui.push_id(idx.id() as i32);
            let selected = self.node_states
                .entry(*idx)
                .or_insert_with(Default::default)
                .selected;
            if ui.selectable(
                &ImString::new(node.name),
                selected,
                ImGuiSelectableFlags::empty(),
                (0.0, 0.0),
            ) {
                if !ui.imgui().key_ctrl() {
                    for state in self.node_states.values_mut() {
                        state.selected = false;
                    }
                }
                let state = self.node_states.get_mut(idx).unwrap();
                state.selected = !state.selected;
                self.active_node = Some(*idx);
            }
            ui.pop_id();
        }
    }

    fn render_graph_node(&mut self, ui: &Ui) {
        let is_mouse_being_dragged_for_scrolling =
            ui.imgui().is_mouse_dragging(ImMouseButton::Middle);
        ui.child_frame(im_str!("GraphNodeChildWindow"), (0.0, 0.0))
            .build(|| {
                if self.show_top_pane {
                    const TOP_PANE_DESIGN: [StyleVar; 2] = [
                        StyleVar::ItemSpacing(ImVec2 { x: 0.0, y: 0.0 }),
                        StyleVar::ItemInnerSpacing(ImVec2 { x: 0.0, y: 0.0 }),
                    ];
                    ui.with_style_vars(&TOP_PANE_DESIGN, || {
                        ui.checkbox(
                            im_str!("Show connection names."),
                            &mut self.show_connection_names,
                        );
                        ui.same_line_spacing(0.0, 15.0);
                        ui.text(im_str!("Use CTRL+MW to zoom. Scroll with MMB."));
                        ui.same_line(ui.get_window_width() - 120.0);
                        ui.checkbox(im_str!("Show grid"), &mut self.show_grid);
                        ui.text(im_str!("Double-click LMB on slots to remove their links (or SHIFT+LMB on links)."));
                    });
                }
                const GRAPH_STYLE_VAR: [StyleVar; 2] = [
                    StyleVar::FramePadding(ImVec2 { x: 1.0, y: 1.0 }),
                    StyleVar::WindowPadding(ImVec2 { x: 0.0, y: 0.0 }),
                ];
                const GRAPH_STYLE_COLOR: [(ImGuiCol, (f32, f32, f32, f32)); 1] =
                    [(ImGuiCol::ChildWindowBg, (0.24, 0.24, 0.27, 0.78))];
                ui.with_style_and_color_vars(&GRAPH_STYLE_VAR, &GRAPH_STYLE_COLOR, || {
                    ui.child_frame(im_str!("scrolling_region"), (0.0, 0.0))
                        .show_borders(true)
                        .show_scrollbar(false)
                        .movable(false)
                        .show_scrollbar_with_mouse(false)
                        .build(|| {
                            // TODO: Manage scaling (and font-scaling)
                            self.render_graph_canvas(ui);
                        });
                });
            });
    }

    fn render_graph_canvas(&mut self, ui: &Ui) {
        const CURRENT_FONT_WINDOW_SCALE: f32 = 1.0;
        const NODE_SLOT_RADIUS: f32 = 5.0 * CURRENT_FONT_WINDOW_SCALE;
        const NODE_SLOT_RADIUS_SQUARED: f32 = NODE_SLOT_RADIUS * NODE_SLOT_RADIUS;
        const NODE_WINDOW_PADDING: ImVec2 = ImVec2 { x: 0.0, y: 0.0 };
        let mouse_delta_squared = {
            let delta = ui.imgui().mouse_delta();
            delta.0 * delta.0 + delta.1 * delta.1
        };
        // We don't detect "mouse release" events while dragging links onto slots.
        // Instead we check that our mouse delta is small enough. Otherwise we couldn't
        // hover other slots while dragging links.
        const MOUSE_DELTA_SQUARED_THRESHOLD: f32 = NODE_SLOT_RADIUS_SQUARED * 0.05;
        const BASE_NODE_WIDTH: f32 = 120.0 * CURRENT_FONT_WINDOW_SCALE;
        let mut current_node_width = BASE_NODE_WIDTH;
        ui.with_item_width(current_node_width, || {
            ui.with_window_draw_list(|list| {
                list.channels_split(5, |draw_list| {
                    let canvas_size = ui.get_window_size();
                    let win_pos = ui.get_cursor_screen_pos();
                    // TODO: Center view on a specific node
                    let effective_scrolling = (
                        self.scrolling.0 - canvas_size.0 * 0.5,
                        self.scrolling.1 - canvas_size.1 * 0.5,
                    );
                    let offset = (
                        win_pos.0 - effective_scrolling.0,
                        win_pos.1 - effective_scrolling.1,
                    );

                    if self.show_grid {
                        let (cursor_pos_x, cursor_pos_y) = ui.get_cursor_pos();
                        let offset2 = (
                            cursor_pos_x - effective_scrolling.0,
                            cursor_pos_y - effective_scrolling.1,
                        );
                        const GRID_COLOR: [f32; 4] = [0.78, 0.78, 0.78, 0.16];
                        const GRID_SIZE: f32 = 64.0;
                        const GRID_LINE_WIDTH: f32 = 1.0;
                        let grid_sz = CURRENT_FONT_WINDOW_SCALE * GRID_SIZE;
                        let grid_line_width = CURRENT_FONT_WINDOW_SCALE * GRID_LINE_WIDTH;
                        let mut x = offset2.0 % grid_sz;
                        while x < canvas_size.0 {
                            let p1 = (x + win_pos.0, win_pos.1);
                            let p2 = (x + win_pos.0, canvas_size.1 + win_pos.1);
                            draw_list
                                .add_line(p1, p2, GRID_COLOR)
                                .thickness(grid_line_width)
                                .build();
                            x += grid_sz;
                        }
                        let mut y = offset2.1 % grid_sz;
                        while y < canvas_size.1 {
                            let p1 = (win_pos.0, y + win_pos.1);
                            let p2 = (canvas_size.0 + win_pos.0, y + win_pos.1);
                            draw_list
                                .add_line(p1, p2, GRID_COLOR)
                                .thickness(grid_line_width)
                                .build();
                            y += grid_sz;
                        }
                    }
                })
            });
        });
    }
}
