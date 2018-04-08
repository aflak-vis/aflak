extern crate aflak_cake as cake;
#[macro_use]
extern crate imgui;
extern crate imgui_sys as sys;

use cake::{Transformation, DST};
use imgui::{ImGuiCol, ImGuiCond, ImGuiMouseCursor, ImGuiSelectableFlags, ImMouseButton, ImString,
            ImVec2, StyleVar, Ui};
use std::collections::BTreeMap;
use std::fmt;

type NodeStates = BTreeMap<cake::NodeId, NodeState>;

pub struct NodeEditor<'t, T: 't + Clone, E: 't, ED> {
    dst: DST<'t, T, E>,
    addable_nodes: &'t [&'t Transformation<T, E>],
    node_states: NodeStates,
    active_node: Option<cake::NodeId>,
    drag_node: Option<cake::NodeId>,
    creating_link: Option<LinkExtremity>,
    new_link: Option<(cake::Output, InputSlot)>,
    pub show_left_pane: bool,
    left_pane_size: Option<f32>,
    pub show_top_pane: bool,
    pub show_connection_names: bool,
    scrolling: ImVec2,
    pub show_grid: bool,
    constant_editor: ED,
}

enum LinkExtremity {
    Output(cake::Output),
    Input(InputSlot),
}

#[derive(Copy, Clone)]
enum InputSlot {
    Transform(cake::Input),
    Output(cake::OutputId),
}

impl<'t, T: Clone, E, ED: Default> Default for NodeEditor<'t, T, E, ED> {
    fn default() -> Self {
        Self {
            dst: DST::new(),
            addable_nodes: &[],
            node_states: BTreeMap::new(),
            active_node: None,
            drag_node: None,
            creating_link: None,
            new_link: None,
            show_left_pane: true,
            left_pane_size: None,
            show_top_pane: true,
            show_connection_names: true,
            scrolling: ImVec2::new(0.0, 0.0),
            show_grid: true,
            constant_editor: ED::default(),
        }
    }
}

#[derive(Debug)]
struct NodeState {
    selected: bool,
    open: bool,
    pos: ImVec2,
    size: ImVec2,
}

impl Default for NodeState {
    fn default() -> Self {
        Self {
            selected: false,
            open: true,
            pos: ImVec2::new(0.0, 0.0),
            size: ImVec2::new(0.0, 0.0),
        }
    }
}

impl NodeState {
    fn get_pos(&self, font_window_scale: f32) -> ImVec2 {
        self.pos * font_window_scale
    }

    fn get_input_slot_pos<I: Into<usize>, C: Into<usize>>(
        &self,
        slot_idx: I,
        slot_cnt: C,
        font_window_scale: f32,
    ) -> ImVec2 {
        ImVec2::new(
            self.pos.x * font_window_scale,
            self.pos.y * font_window_scale
                + self.size.y * (slot_idx.into() + 1) as f32 / (slot_cnt.into() + 1) as f32,
        )
    }

    fn get_output_slot_pos<I: Into<usize>, C: Into<usize>>(
        &self,
        slot_idx: I,
        slot_cnt: C,
        font_window_scale: f32,
    ) -> ImVec2 {
        ImVec2::new(
            self.pos.x * font_window_scale + self.size.x,
            self.pos.y * font_window_scale
                + self.size.y * (slot_idx.into() + 1) as f32 / (slot_cnt.into() + 1) as f32,
        )
    }
}

impl<'t, T, E, ED> NodeEditor<'t, T, E, ED>
where
    T: Clone + cake::VariantName + Send + Sync,
    E: Send,
{
    pub fn compute_output(&self, id: &cake::OutputId) -> Result<T, cake::DSTError<E>> {
        self.dst.compute(id)
    }
}

impl<'t, T, E, ED> NodeEditor<'t, T, E, ED>
where
    T: Clone + cake::EditableVariants + cake::VariantName + cake::DefaultFor,
    ED: ConstantEditor<T>,
    E: fmt::Debug,
{
    pub fn new(addable_nodes: &'t [&'t Transformation<T, E>], ed: ED) -> Self {
        Self {
            addable_nodes,
            constant_editor: ed,
            ..Default::default()
        }
    }

    pub fn from_dst(
        dst: DST<'t, T, E>,
        addable_nodes: &'t [&'t Transformation<T, E>],
        ed: ED,
    ) -> Self {
        Self {
            dst,
            addable_nodes,
            constant_editor: ed,
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

    pub fn outputs(&self) -> Vec<cake::OutputId> {
        self.dst
            .outputs_iter()
            .filter(|(_, some_output)| some_output.is_some())
            .map(|(id, _)| *id)
            .collect()
    }

    fn render_left_pane(&mut self, ui: &Ui) {
        const LEFT_PANE_DEFAULT_RELATIVE_WIDTH: f32 = 0.2;
        let window_size = ui.get_window_size();
        let pane_width = *self.left_pane_size
            .get_or_insert_with(|| window_size.x * LEFT_PANE_DEFAULT_RELATIVE_WIDTH);

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
                let maxw = minw + window_size.x - SPLITTER_WIDTH - style.window_min_size.x;
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
        for (idx, node) in self.dst.transforms_outputs_iter() {
            //ui.push_id(idx.id() as i32);
            if !self.node_states.contains_key(&idx) {
                let new_node = self.init_node();
                self.node_states.insert(idx, new_node);
            }
            let selected = self.node_states.get(&idx).unwrap().selected;
            let name = ImString::new(node.name(&idx));
            if ui.selectable(&name, selected, ImGuiSelectableFlags::empty(), (0.0, 0.0)) {
                if !ui.imgui().key_ctrl() {
                    deselect_all_nodes(&mut self.node_states);
                }
                toggle_select_node(&mut self.node_states, &idx);
                self.active_node = Some(idx);
            }
            //ui.pop_id();
        }
    }

    fn render_graph_node(&mut self, ui: &Ui) {
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
        // We don't detect "mouse release" events while dragging links onto slots.
        // Instead we check that our mouse delta is small enough. Otherwise we couldn't
        // hover other slots while dragging links.
        const BASE_NODE_WIDTH: f32 = 120.0 * CURRENT_FONT_WINDOW_SCALE;
        ui.with_item_width(BASE_NODE_WIDTH, || {
            ui.with_window_draw_list(|list| {
                list.channels_split(5, |draw_list| {
                    let canvas_size = ui.get_window_size();
                    let win_pos = ui.get_cursor_screen_pos();
                    // TODO: Center view on a specific node
                    let effective_scrolling = self.scrolling - canvas_size * 0.5;
                    let offset = win_pos - effective_scrolling;

                    if self.show_grid {
                        let cursor_pos = ui.get_cursor_pos();
                        let offset2 = cursor_pos - effective_scrolling;
                        const GRID_COLOR: [f32; 4] = [0.78, 0.78, 0.78, 0.16];
                        const GRID_SIZE: f32 = 64.0;
                        const GRID_LINE_WIDTH: f32 = 1.0;
                        let grid_sz = CURRENT_FONT_WINDOW_SCALE * GRID_SIZE;
                        let grid_line_width = CURRENT_FONT_WINDOW_SCALE * GRID_LINE_WIDTH;
                        let mut x = offset2.x % grid_sz;
                        while x < canvas_size.x {
                            let p1 = ImVec2::new(x + win_pos.x, win_pos.y);
                            let p2 = (x + win_pos.x, canvas_size.y + win_pos.y);
                            draw_list
                                .add_line(p1, p2, GRID_COLOR)
                                .thickness(grid_line_width)
                                .build();
                            x += grid_sz;
                        }
                        let mut y = offset2.y % grid_sz;
                        while y < canvas_size.y {
                            let p1 = (win_pos.x, y + win_pos.y);
                            let p2 = (canvas_size.x + win_pos.x, y + win_pos.y);
                            draw_list
                                .add_line(p1, p2, GRID_COLOR)
                                .thickness(grid_line_width)
                                .build();
                            y += grid_sz;
                        }
                    }

                    // Bezier control point of the links
                    const LINK_CONTROL_POINT_DISTANCE: f32 = 50.0;
                    let link_cp =
                        ImVec2::new(LINK_CONTROL_POINT_DISTANCE * CURRENT_FONT_WINDOW_SCALE, 0.0);
                    const LINK_LINE_WIDTH: f32 = 3.0;
                    let link_line_width = LINK_LINE_WIDTH * CURRENT_FONT_WINDOW_SCALE;
                    // NODE LINK CULLING?

                    for idx in self.dst.node_ids() {
                        let node_pos = node_state_get(&self.node_states, &idx, |state| {
                            state.get_pos(CURRENT_FONT_WINDOW_SCALE)
                        });
                        // ui.push_id(idx.id() as i32);

                        // Display node contents first in the foreground
                        draw_list.channels_set_current(if self.active_node == Some(idx) {
                            4
                        } else {
                            2
                        });

                        let node_rect_min = offset + node_pos;
                        let node_rect_max = node_state_get(&self.node_states, &idx, |state| {
                            node_rect_min + state.size
                        });
                        ui.set_cursor_screen_pos(node_rect_min + NODE_WINDOW_PADDING);
                        self.draw_node_inside(ui, &idx); // ...

                        let node = self.dst.get_node(&idx).unwrap();
                        let node_states = &mut self.node_states;
                        let item_rect_size = ui.get_item_rect_size();
                        node_state_set(node_states, &idx, |state| {
                            state.size = item_rect_size + NODE_WINDOW_PADDING * 2.0;
                        });

                        draw_list.channels_set_current(if self.active_node == Some(idx) {
                            3
                        } else {
                            1
                        });
                        ui.set_cursor_screen_pos(node_rect_min);
                        ui.invisible_button(
                            im_str!("node##nodeinvbtn"),
                            node_state_get(node_states, &idx, |state| state.size),
                        );
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
                        let line_thickness = if self.active_node == Some(idx) {
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
                        if node_state_get(node_states, &idx, |state| state.open) {
                            let node_title_bar_height =
                                ui.get_text_line_height_with_spacing() + NODE_WINDOW_PADDING.y;
                            let tmp1 = ImVec2::new(
                                node_rect_min.x,
                                node_rect_min.y + node_title_bar_height + 1.0,
                            );
                            let tmp2 = (node_rect_max.x, tmp1.y);
                            draw_list
                                .add_line(tmp1, tmp2, NODE_FRAME_COLOR)
                                .thickness(line_thickness)
                                .build();
                        }
                        // Display connectors
                        const CONNECTOR_BORDER_THICKNESS: f32 = NODE_SLOT_RADIUS * 0.25;
                        const INPUT_SLOT_COLOR: [f32; 4] = [0.59, 0.59, 0.59, 0.59];
                        for (slot_idx, &slot_name) in node.inputs_iter().enumerate() {
                            let connector_pos = node_state_get(node_states, &idx, |state| {
                                state.get_input_slot_pos(
                                    slot_idx,
                                    node.inputs_count(),
                                    CURRENT_FONT_WINDOW_SCALE,
                                )
                            });
                            let connector_screen_pos = offset + connector_pos;
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
                                    connector_screen_pos.x - NODE_SLOT_RADIUS - name_size.x,
                                    connector_screen_pos.y - name_size.y,
                                ));
                                ui.text(&ImString::new(slot_name));
                            }
                            if ui.imgui().is_mouse_clicked(ImMouseButton::Left) {
                                let mouse_pos: ImVec2 = ui.imgui().mouse_pos().into();
                                if (mouse_pos - connector_screen_pos).squared_norm()
                                    <= NODE_SLOT_RADIUS_SQUARED
                                {
                                    self.drag_node = None;
                                    self.creating_link = Some(LinkExtremity::Input(match idx {
                                        cake::NodeId::Transform(t_idx) => {
                                            InputSlot::Transform(cake::Input::new(t_idx, slot_idx))
                                        }
                                        cake::NodeId::Output(output_id) => {
                                            InputSlot::Output(output_id)
                                        }
                                    }));
                                }
                            }
                            if let Some(LinkExtremity::Output(link_output)) = self.creating_link {
                                // Check if we hover slot!
                                let mouse_pos: ImVec2 = ui.imgui().mouse_pos().into();
                                if (mouse_pos - connector_screen_pos).squared_norm()
                                    <= NODE_SLOT_RADIUS_SQUARED
                                {
                                    self.new_link = Some((
                                        link_output,
                                        match idx {
                                            cake::NodeId::Transform(t_idx) => InputSlot::Transform(
                                                cake::Input::new(t_idx, slot_idx),
                                            ),
                                            cake::NodeId::Output(output_id) => {
                                                InputSlot::Output(output_id)
                                            }
                                        },
                                    ));
                                    self.creating_link = None;
                                }
                            }
                        }

                        // Show outputs for transform nodes
                        if let cake::NodeId::Transform(t_idx) = idx {
                            const OUTPUT_SLOT_COLOR: [f32; 4] = [0.59, 0.59, 0.59, 0.59];
                            for (slot_idx, &slot_name) in node.outputs_iter().enumerate() {
                                let connector_pos = node_state_get(node_states, &idx, |state| {
                                    state.get_output_slot_pos(
                                        slot_idx,
                                        node.outputs_count(),
                                        CURRENT_FONT_WINDOW_SCALE,
                                    )
                                });
                                let connector_screen_pos = offset + connector_pos;
                                draw_list
                                    .add_circle(
                                        connector_screen_pos,
                                        NODE_SLOT_RADIUS,
                                        OUTPUT_SLOT_COLOR,
                                    )
                                    .thickness(CONNECTOR_BORDER_THICKNESS)
                                    .filled(true)
                                    .build();
                                if self.show_connection_names {
                                    let name_size =
                                        ui.calc_text_size(&ImString::new(slot_name), false, -1.0);
                                    ui.set_cursor_screen_pos((
                                        connector_screen_pos.x + NODE_SLOT_RADIUS,
                                        connector_screen_pos.y - name_size.y,
                                    ));
                                    ui.text(&ImString::new(slot_name));
                                }
                                if ui.imgui().is_mouse_clicked(ImMouseButton::Left) {
                                    let mouse_pos: ImVec2 = ui.imgui().mouse_pos().into();
                                    if (mouse_pos - connector_screen_pos).squared_norm()
                                        <= NODE_SLOT_RADIUS_SQUARED
                                    {
                                        self.drag_node = None;
                                        self.creating_link = Some(LinkExtremity::Output(
                                            cake::Output::new(t_idx, slot_idx),
                                        ));
                                    }
                                }
                                if let Some(LinkExtremity::Input(link_input)) = self.creating_link {
                                    // Check if we hover slot!
                                    let mouse_pos: ImVec2 = ui.imgui().mouse_pos().into();
                                    if (mouse_pos - connector_screen_pos).squared_norm()
                                        <= NODE_SLOT_RADIUS_SQUARED
                                    {
                                        self.new_link =
                                            Some((cake::Output::new(t_idx, slot_idx), link_input));
                                        self.creating_link = None;
                                    }
                                }
                            }
                            //ui.pop_id();
                        }
                    }
                    // Preview new link
                    const NEW_LINK_COLOR: [f32; 3] = [0.78, 0.78, 0.39];
                    if self.creating_link.is_some() {
                        if ui.imgui().is_mouse_dragging(ImMouseButton::Left) {
                            let (p1, cp1, cp2, p2) = match self.creating_link.as_ref().unwrap() {
                                &LinkExtremity::Output(output) => {
                                    let output_node_count =
                                        self.dst.get_transform(&output.t_idx).unwrap().output.len();
                                    let output_node_state = self.node_states
                                        .get(&cake::NodeId::Transform(output.t_idx))
                                        .unwrap();
                                    let connector_pos = output_node_state.get_output_slot_pos(
                                        output.index(),
                                        output_node_count,
                                        CURRENT_FONT_WINDOW_SCALE,
                                    );
                                    let p1 = offset + connector_pos;
                                    let p2: ImVec2 = ui.imgui().mouse_pos().into();
                                    let cp1 = p1 + link_cp;
                                    let cp2 = p2 - link_cp;
                                    (p1, cp1, cp2, p2)
                                }
                                &LinkExtremity::Input(input_slot) => {
                                    let connector_pos = match input_slot {
                                        InputSlot::Transform(input) => {
                                            let input_node_count = self.dst
                                                .get_transform(&input.t_idx)
                                                .unwrap()
                                                .input
                                                .len();
                                            let input_node_state = self.node_states
                                                .get(&cake::NodeId::Transform(input.t_idx))
                                                .unwrap();
                                            input_node_state.get_input_slot_pos(
                                                input.index(),
                                                input_node_count,
                                                CURRENT_FONT_WINDOW_SCALE,
                                            )
                                        }
                                        InputSlot::Output(output_id) => {
                                            let input_node_state = self.node_states
                                                .get(&cake::NodeId::Output(output_id))
                                                .unwrap();
                                            input_node_state.get_input_slot_pos(
                                                0usize,
                                                1usize,
                                                CURRENT_FONT_WINDOW_SCALE,
                                            )
                                        }
                                    };
                                    let p1 = offset + connector_pos;
                                    let p2: ImVec2 = ui.imgui().mouse_pos().into();
                                    let cp1 = p1 - link_cp;
                                    let cp2 = p2 + link_cp;
                                    (p1, cp1, cp2, p2)
                                }
                            };
                            draw_list
                                .add_bezier_curve(p1, cp1, cp2, p2, NEW_LINK_COLOR)
                                .thickness(link_line_width)
                                .build();
                        }
                        if !ui.imgui().is_mouse_down(ImMouseButton::Left) {
                            self.creating_link = None;
                        }
                    }

                    // Display links
                    draw_list.channels_set_current(0);
                    for (output, input_slot) in self.dst.links_iter() {
                        let connector_in_pos = match input_slot {
                            cake::InputSlot::Transform(input) => {
                                let input_node_count =
                                    self.dst.get_transform(&input.t_idx).unwrap().input.len();
                                let input_node_state = self.node_states
                                    .get(&cake::NodeId::Transform(input.t_idx))
                                    .unwrap();
                                input_node_state.get_input_slot_pos(
                                    input.index(),
                                    input_node_count,
                                    CURRENT_FONT_WINDOW_SCALE,
                                )
                            }
                            cake::InputSlot::Output(output_id) => {
                                let input_node_state = self.node_states
                                    .get(&cake::NodeId::Output(*output_id))
                                    .unwrap();
                                input_node_state.get_input_slot_pos(
                                    0usize,
                                    1usize,
                                    CURRENT_FONT_WINDOW_SCALE,
                                )
                            }
                        };
                        let p1 = offset + connector_in_pos;
                        let output_node_count =
                            self.dst.get_transform(&output.t_idx).unwrap().output.len();
                        let output_node_state = self.node_states
                            .get(&cake::NodeId::Transform(output.t_idx))
                            .unwrap();

                        let connector_out_pos = output_node_state.get_output_slot_pos(
                            output.index(),
                            output_node_count,
                            CURRENT_FONT_WINDOW_SCALE,
                        );
                        let p2 = offset + connector_out_pos;
                        let cp1 = p1 - link_cp;
                        let cp2 = p2 + link_cp;
                        const LINK_COLOR: [f32; 3] = [0.78, 0.78, 0.39];
                        draw_list
                            .add_bezier_curve(p1, cp1, cp2, p2, LINK_COLOR)
                            .thickness(link_line_width)
                            .build();
                    }
                })
            });
        });
        if let Some((output, input_slot)) = self.new_link {
            match input_slot {
                InputSlot::Transform(input) => {
                    if let Err(e) = self.dst.connect(output, input) {
                        // TODO: Make a modal to show error
                        eprintln!("{:?}", e);
                    }
                }
                InputSlot::Output(output_id) => self.dst.update_output(output_id, output),
            }
            self.new_link = None;
        }
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
            ui.separator();
            if ui.menu_item(im_str!("Output node")).build() {
                self.dst.create_output();
            }
            ui.separator();
            for constant_type in T::editable_variants() {
                let item_name = ImString::new(format!("Input node: {}", constant_type));
                if ui.menu_item(&item_name).build() {
                    let constant = Transformation::new_constant(T::default_for(constant_type));
                    self.dst.add_owned_transform(constant);
                }
            }
        });
    }

    fn draw_node_inside(&mut self, ui: &Ui, id: &cake::NodeId) {
        let node_name = {
            let node = self.dst.get_node(id).unwrap();
            ImString::new(node.name(id))
        };
        let node_states = &mut self.node_states;
        let dst = &mut self.dst;
        let constant_editor = &self.constant_editor;
        ui.group(|| {
            let default_text_color = ui.imgui().style().colors[ImGuiCol::Text as usize];
            ui.with_color_var(ImGuiCol::Text, default_text_color, || {
                const TRANSPARENT: [f32; 4] = [1.0, 1.0, 1.0, 0.0];
                const TREE_STYLE: [(ImGuiCol, [f32; 4]); 3] = [
                    (ImGuiCol::Header, TRANSPARENT),
                    (ImGuiCol::HeaderActive, TRANSPARENT),
                    (ImGuiCol::HeaderHovered, TRANSPARENT),
                ];
                ui.with_color_vars(&TREE_STYLE, || {
                    if ui.tree_node(&node_name)
                        .opened(
                            node_state_get(node_states, id, |state| state.open),
                            ImGuiCond::Always,
                        )
                        .build(|| {})
                    {
                        open_node(node_states, id, false);
                    } else {
                        open_node(node_states, id, true);
                    }
                });
                ui.same_line_spacing(0.0, 2.0);
                //ui.text(node.name);
                if ui.is_item_hovered() {
                    // Show tooltip ?
                    ui.tooltip(|| ui.text("TEST TOOLTIP"));
                }
            });
            let mut constant_editor_in_use = false;
            if let &cake::NodeId::Transform(ref t_idx) = id {
                if let Some(t) = dst.get_transform_mut(t_idx) {
                    if let cake::Algorithm::Constant(ref mut constants) = t.algorithm {
                        ui.dummy([0.0, 20.0]);
                        for c in constants.iter_mut() {
                            constant_editor.editor(ui, c);
                            constant_editor_in_use = true;
                        }
                    }
                }
            }
            if !constant_editor_in_use {
                ui.dummy([0.0, 100.0]);
            }
            // TODO: Add copy-paste buttons
        });
        if ui.is_item_hovered() {
            if ui.imgui().is_mouse_clicked(ImMouseButton::Left) {
                self.active_node = Some(*id);
                self.drag_node = Some(*id);
                if !ui.imgui().key_ctrl() {
                    deselect_all_nodes(node_states);
                }
                toggle_select_node(node_states, id);
            }
        }
        if self.drag_node == Some(*id) {
            if ui.imgui().is_mouse_dragging(ImMouseButton::Left) {
                let delta = ui.imgui().mouse_delta();
                node_state_set(node_states, id, |state| {
                    state.pos = state.pos + delta.into();
                });
            } else if !ui.imgui().is_mouse_down(ImMouseButton::Left) {
                self.drag_node = None;
            }
        }
    }
}

/// Manage nodes
impl<'t, T: Clone, E, ED> NodeEditor<'t, T, E, ED> {
    fn init_node(&self) -> NodeState {
        let mut max = -300.0;
        for state in self.node_states.values() {
            if state.pos.y > max {
                max = state.pos.y;
            }
        }
        NodeState {
            pos: ImVec2::new(0.0, max + 150.0),
            ..Default::default()
        }
    }
}

fn deselect_all_nodes(node_states: &mut NodeStates) {
    for state in node_states.values_mut() {
        state.selected = false;
    }
}

fn toggle_select_node(node_states: &mut NodeStates, id: &cake::NodeId) {
    let state = node_states.get_mut(id).unwrap();
    state.selected = !state.selected;
}

fn open_node(node_states: &mut NodeStates, idx: &cake::NodeId, open: bool) {
    let state = node_states.get_mut(idx).unwrap();
    state.open = open;
}

fn node_state_get<T, F: FnOnce(&NodeState) -> T>(
    node_states: &NodeStates,
    id: &cake::NodeId,
    f: F,
) -> T {
    let state = node_states.get(id).unwrap();
    f(state)
}

fn node_state_set<T, F: FnOnce(&mut NodeState) -> T>(
    node_states: &mut NodeStates,
    id: &cake::NodeId,
    f: F,
) -> T {
    let state = node_states.get_mut(id).unwrap();
    f(state)
}

pub trait ConstantEditor<T>: Default {
    fn editor(&self, ui: &Ui, constant: &mut T);
}
