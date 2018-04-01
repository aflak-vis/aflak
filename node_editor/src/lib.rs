extern crate aflak_cake as cake;
#[macro_use]
extern crate imgui;
extern crate imgui_sys as sys;

use std::collections::BTreeMap;
use cake::{TransformIdx, Transformation, DST};
use imgui::{ImGuiCol, ImGuiCond, ImGuiMouseCursor, ImGuiSelectableFlags, ImMouseButton, ImStr,
            ImString, ImVec2, StyleVar, Ui};

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

#[derive(Debug)]
struct NodeState {
    selected: bool,
    open: bool,
    pos: (f32, f32),
    size: (f32, f32),
}

impl Default for NodeState {
    fn default() -> Self {
        Self {
            selected: false,
            open: true,
            pos: (0.0, 0.0),
            size: (0.0, 0.0),
        }
    }
}

impl NodeState {
    fn get_pos(&self, font_window_scale: f32) -> (f32, f32) {
        (
            self.pos.0 * font_window_scale,
            self.pos.1 * font_window_scale,
        )
    }

    fn get_input_slot_pos<I: Into<usize>, C: Into<usize>>(
        &self,
        slot_idx: I,
        slot_cnt: C,
        font_window_scale: f32,
    ) -> (f32, f32) {
        (
            self.pos.0 * font_window_scale,
            self.pos.1 * font_window_scale
                + self.size.1 * (slot_idx.into() + 1) as f32 / (slot_cnt.into() + 1) as f32,
        )
    }

    fn get_output_slot_pos<I: Into<usize>, C: Into<usize>>(
        &self,
        slot_idx: I,
        slot_cnt: C,
        font_window_scale: f32,
    ) -> (f32, f32) {
        (
            self.pos.0 * font_window_scale + self.size.0,
            self.pos.1 * font_window_scale
                + self.size.1 * (slot_idx.into() + 1) as f32 / (slot_cnt.into() + 1) as f32,
        )
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
            if !self.node_states.contains_key(idx) {
                let new_node = self.init_node();
                self.node_states.insert(*idx, new_node);
            }
            let selected = self.node_states.get(idx).unwrap().selected;
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

                    // Bezier control point of the links
                    const LINK_CONTROL_POINT_DISTANCE: f32 = 50.0;
                    let link_cp = [LINK_CONTROL_POINT_DISTANCE * CURRENT_FONT_WINDOW_SCALE, 0.0];
                    const LINK_LINE_WIDTH: f32 = 3.0;
                    let link_line_width = LINK_LINE_WIDTH * CURRENT_FONT_WINDOW_SCALE;
                    // NODE LINK CULLING?

                    for (idx, node) in self.dst.transforms_iter() {
                        let state = self.node_states.get_mut(idx).unwrap();
                        let node_pos = state.get_pos(CURRENT_FONT_WINDOW_SCALE);
                        ui.push_id(idx.id() as i32);

                        // Display node contents first in the foreground
                        draw_list.channels_set_current(if self.active_node == Some(*idx) {
                            4
                        } else {
                            2
                        });

                        let node_rect_min = (offset.0 + node_pos.0, offset.1 + node_pos.1);
                        let node_rect_max = (
                            node_rect_min.0 + state.size.0,
                            node_rect_min.1 + state.size.1,
                        );
                        ui.set_cursor_screen_pos((
                            node_rect_min.0 + NODE_WINDOW_PADDING.x,
                            node_rect_min.1 + NODE_WINDOW_PADDING.y,
                        ));
                        ui.group(|| {
                            let default_text_color =
                                ui.imgui().style().colors[ImGuiCol::Text as usize];
                            ui.with_color_var(ImGuiCol::Text, default_text_color, || {
                                const TRANSPARENT: [f32; 4] = [1.0, 1.0, 1.0, 0.0];
                                const TREE_STYLE: [(ImGuiCol, [f32; 4]); 3] = [
                                    (ImGuiCol::Header, TRANSPARENT),
                                    (ImGuiCol::HeaderActive, TRANSPARENT),
                                    (ImGuiCol::HeaderHovered, TRANSPARENT),
                                ];
                                ui.with_color_vars(&TREE_STYLE, || {
                                    if ui.tree_node(&ImString::new(node.name))
                                        .opened(state.open, ImGuiCond::Always)
                                        .build(|| {})
                                    {
                                        state.open = false;
                                    } else {
                                        state.open = true;
                                    }
                                });
                                ui.same_line_spacing(0.0, 2.0);
                                //ui.text(node.name);
                                if ui.is_item_hovered() {
                                    // Show tooltip ?
                                    ui.tooltip(|| ui.text("TEST TOOLTIP"));
                                }
                            });
                            // TODO: Add copy-paste buttons
                        });

                        let item_rect_size = ui.get_item_rect_size();
                        state.size = (
                            item_rect_size.0 + 2.0 * NODE_WINDOW_PADDING.x,
                            item_rect_size.1 + 2.0 * NODE_WINDOW_PADDING.y,
                        );

                        draw_list.channels_set_current(if self.active_node == Some(*idx) {
                            3
                        } else {
                            1
                        });
                        ui.set_cursor_screen_pos(node_rect_min);
                        ui.invisible_button(im_str!("node##nodeinvbtn"), state.size);
                        // TODO: Handle selection

                        const NODE_ROUNDING: f32 = 4.0;
                        const NODE_COLOR: [f32; 3] = [0.24, 0.24, 0.24];
                        let node_bg_color = NODE_COLOR;
                        draw_list
                            .add_rect(node_rect_min, node_rect_max, node_bg_color)
                            .rounding(NODE_ROUNDING)
                            .filled(true)
                            .build();

                        // Display frame
                        const NODE_FRAME_COLOR: [f32; 3] = [0.39, 0.39, 0.39];
                        let line_thickness = if self.active_node == Some(*idx) {
                            3.0
                        } else {
                            1.0
                        } * CURRENT_FONT_WINDOW_SCALE;
                        draw_list
                            .add_rect(node_rect_min, node_rect_max, NODE_FRAME_COLOR)
                            .thickness(line_thickness)
                            .rounding(NODE_ROUNDING)
                            .build();
                        // Line below node name
                        if state.open {
                            let node_title_bar_height =
                                ui.get_text_line_height_with_spacing() + NODE_WINDOW_PADDING.y;
                            let tmp1 = (
                                node_rect_min.0,
                                node_rect_min.1 + node_title_bar_height + 1.0,
                            );
                            let tmp2 = (node_rect_max.0, tmp1.1);
                            draw_list
                                .add_line(tmp1, tmp2, NODE_FRAME_COLOR)
                                .thickness(line_thickness)
                                .build();
                        }
                        // Display connectors
                        const CONNECTOR_BORDER_THICKNESS: f32 = NODE_SLOT_RADIUS * 0.25;
                        const INPUT_SLOT_COLOR: [f32; 4] = [0.59, 0.59, 0.59, 0.59];
                        for (slot_idx, &slot_name) in node.input.iter().enumerate() {
                            let connector_pos = state.get_input_slot_pos(
                                slot_idx,
                                node.input.len(),
                                CURRENT_FONT_WINDOW_SCALE,
                            );
                            let connector_screen_pos =
                                (offset.0 + connector_pos.0, offset.1 + connector_pos.1);
                            draw_list
                                .add_circle(
                                    connector_screen_pos,
                                    NODE_SLOT_RADIUS,
                                    INPUT_SLOT_COLOR,
                                )
                                .thickness(CONNECTOR_BORDER_THICKNESS)
                                .filled(true)
                                .build();
                            if self.show_connection_names {
                                let name_size =
                                    ui.calc_text_size(&ImString::new(slot_name), false, -1.0);
                                ui.set_cursor_screen_pos((
                                    connector_screen_pos.0 - NODE_SLOT_RADIUS - name_size.x,
                                    connector_screen_pos.1 - name_size.y,
                                ));
                                ui.text(&ImString::new(slot_name));
                            }
                        }
                        const OUTPUT_SLOT_COLOR: [f32; 4] = [0.59, 0.59, 0.59, 0.59];
                        for (slot_idx, &slot_name) in node.output.iter().enumerate() {
                            let connector_pos = state.get_output_slot_pos(
                                slot_idx,
                                node.output.len(),
                                CURRENT_FONT_WINDOW_SCALE,
                            );
                            let connector_screen_pos =
                                (offset.0 + connector_pos.0, offset.1 + connector_pos.1);
                            draw_list
                                .add_circle(
                                    connector_screen_pos,
                                    NODE_SLOT_RADIUS,
                                    INPUT_SLOT_COLOR,
                                )
                                .thickness(CONNECTOR_BORDER_THICKNESS)
                                .filled(true)
                                .build();
                            if self.show_connection_names {
                                let name_size =
                                    ui.calc_text_size(&ImString::new(slot_name), false, -1.0);
                                ui.set_cursor_screen_pos((
                                    connector_screen_pos.0 + NODE_SLOT_RADIUS,
                                    connector_screen_pos.1 - name_size.y,
                                ));
                                ui.text(&ImString::new(slot_name));
                            }
                        }
                        ui.pop_id();
                    }

                    // Display links
                    draw_list.channels_set_current(0);
                    for (output, input) in self.dst.edges_iter() {
                        let input_node_count = self.dst.get_transform(&input.t_idx).unwrap().input.len();
                        let output_node_count = self.dst.get_transform(&output.t_idx).unwrap().output.len();
                        let input_node_state = self.node_states.get(&input.t_idx).unwrap();
                        let output_node_state = self.node_states.get(&output.t_idx).unwrap();
                        let connector_in_pos = input_node_state.get_input_slot_pos(
                            input.index(),
                            input_node_count,
                            CURRENT_FONT_WINDOW_SCALE,
                        );
                        let p1 = (offset.0 + connector_in_pos.0, offset.1 + connector_in_pos.1);

                        let connector_out_pos = output_node_state.get_output_slot_pos(
                            output.index(),
                            output_node_count,
                            CURRENT_FONT_WINDOW_SCALE,
                        );
                        let p2 = (offset.0 + connector_out_pos.0, offset.1 + connector_out_pos.1);
                        let cp1 = (p1.0 - link_cp[0], p1.1 - link_cp[1]);
                        let cp2 = (p2.0 + link_cp[0], p2.1 + link_cp[1]);
                        const LINK_COLOR: [f32; 3] = [0.78, 0.78, 0.39];
                        draw_list
                            .add_bezier_curve(p1, cp1, cp2, p2, LINK_COLOR)
                            .thickness(LINK_LINE_WIDTH)
                            .build();
                    }
                })
            });
        });
        if ui.imgui().is_mouse_clicked(ImMouseButton::Right) {
            ui.open_popup(im_str!("TEST"));
        }
        ui.popup(im_str!("TEST"), || {
            ui.text("Add node");
            ui.separator();
            for (i, node) in self.addable_nodes.iter().enumerate() {
                ui.push_id(i as i32);
                if ui.menu_item(&ImString::new(node.name)).build() {
                    self.dst.add_transform(node);
                }
                ui.pop_id();
            }
        });
    }
}

/// Manage nodes
impl<'t, T: Clone, E> NodeEditor<'t, T, E> {
    fn init_node(&self) -> NodeState {
        let mut max = -300.0;
        for state in self.node_states.values() {
            if state.pos.1 > max {
                max = state.pos.1;
            }
        }
        NodeState {
            pos: (0.0, max + 150.0),
            ..Default::default()
        }
    }
}
