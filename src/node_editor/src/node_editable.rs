use std::collections::BTreeMap;
use std::error;
use std::fs;
use std::io;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::path::Path;

use ron::{de, ser};
use serde::{ser::Serializer, Deserialize, Serialize};

use cake::{
    self, DeserDST, GuardRef, InputSlot, Macro, MacroEvaluationError, MacroHandle, NodeId, Output,
    OutputId, Transformation, DST,
};

use compute::{self, ComputeResult};
use export::{ExportError, ImportError};
use node_state::{NodeState, NodeStates};
use scrolling::Scrolling;
use vec2::Vec2;

pub struct NodeEditor<'t, N, T: 't + Clone, E: 't, ED> {
    inner: N,
    addable_nodes: &'t [&'t Transformation<'t, T, E>],
    pub(crate) node_states: NodeStates,
    active_node: Option<NodeId>,
    drag_node: Option<NodeId>,
    creating_link: Option<LinkExtremity>,
    new_link: Option<(Output, InputSlot)>,
    pub show_left_pane: bool,
    left_pane_size: Option<f32>,
    pub show_top_pane: bool,
    pub show_connection_names: bool,
    pub(crate) scrolling: Scrolling,
    pub show_grid: bool,
    constant_editor: ED,
    error_stack: Vec<Box<error::Error>>,
}

enum LinkExtremity {
    Output(Output),
    Input(InputSlot),
}

impl<'t, N: Default, T: Clone, E, ED: Default> Default for NodeEditor<'t, N, T, E, ED> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
            addable_nodes: &[],
            node_states: NodeStates::new(),
            active_node: None,
            drag_node: None,
            creating_link: None,
            new_link: None,
            show_left_pane: true,
            left_pane_size: None,
            show_top_pane: true,
            show_connection_names: true,
            scrolling: Default::default(),
            show_grid: true,
            constant_editor: ED::default(),
            error_stack: vec![],
        }
    }
}

impl<'t, N: Default, T, E, ED> NodeEditor<'t, N, T, E, ED>
where
    T: Clone,
    ED: Default,
{
    pub fn new(addable_nodes: &'t [&'t Transformation<'t, T, E>], ed: ED) -> Self {
        Self {
            addable_nodes,
            constant_editor: ed,
            ..Default::default()
        }
    }
}

impl<'t, T, E, ED> NodeEditor<'t, DstEditor<'t, T, E>, T, E, ED>
where
    T: Clone,
    ED: Default,
{
    pub fn from_dst(
        dst: DST<'t, T, E>,
        addable_nodes: &'t [&'t Transformation<'t, T, E>],
        ed: ED,
    ) -> Self {
        let mut output_results = BTreeMap::new();
        for (output_id, _) in dst.outputs_iter() {
            output_results.insert(*output_id, compute::new_compute_result());
        }
        Self {
            inner: DstEditor {
                dst,
                output_results,
            },
            addable_nodes,
            constant_editor: ed,
            ..Default::default()
        }
    }
}

pub struct DstEditor<'t, T: 't + Clone, E: 't> {
    dst: DST<'t, T, E>,
    output_results: BTreeMap<OutputId, ComputeResult<T, E>>,
}

impl<'t, T: 't + Clone, E: 't> Default for DstEditor<'t, T, E> {
    fn default() -> Self {
        Self {
            dst: DST::default(),
            output_results: BTreeMap::default(),
        }
    }
}

pub struct MacroEditor<'t, T: 't + Clone, E: 't> {
    macr: Macro<'t, T, E>,
}

pub struct NodeEditorApp<'t, T: 't + Clone, E: 't, ED> {
    main: NodeEditor<'t, DstEditor<'t, T, E>, T, E, ED>,
    macros: BTreeMap<String, NodeEditor<'t, MacroEditor<'t, T, E>, T, E, ED>>,
    error_stack: Vec<Box<error::Error>>,
}

pub trait NodeEditable<'a, 't, T: Clone + 't, E: 't>: Sized {
    type DSTHandle: Deref<Target = DST<'t, T, E>>;
    type DSTHandleMut: DerefMut<Target = DST<'t, T, E>>;

    fn dst(&'a self) -> Self::DSTHandle;
    fn dst_mut(&'a mut self) -> Self::DSTHandleMut;
}

impl<'a, 't: 'a, T: Clone + 't, E: 't> NodeEditable<'a, 't, T, E> for DstEditor<'t, T, E> {
    type DSTHandle = &'a DST<'t, T, E>;
    type DSTHandleMut = &'a mut DST<'t, T, E>;

    fn dst(&self) -> &DST<'t, T, E> {
        &self.dst
    }
    fn dst_mut(&mut self) -> &mut DST<'t, T, E> {
        &mut self.dst
    }
}

impl<'a, 't: 'a, T: Clone + 't, E: 't> NodeEditable<'a, 't, T, E> for MacroEditor<'t, T, E> {
    type DSTHandle = GuardRef<'a, DST<'t, T, E>>;
    type DSTHandleMut = MacroHandle<'a, 't, T, E>;

    fn dst(&self) -> GuardRef<DST<'t, T, E>> {
        self.macr.dst()
    }
    fn dst_mut(&'a mut self) -> MacroHandle<'a, 't, T, E> {
        self.macr.dst_mut()
    }
}

impl<'t, N, T, E, ED> NodeEditor<'t, N, T, E, ED>
where
    T: Clone,
    N: Serialize,
{
    pub fn export_to_buf<W: io::Write>(&self, w: &mut W) -> Result<(), ExportError> {
        let serialized = ser::to_string_pretty(self, Default::default())?;
        w.write_all(serialized.as_bytes())?;
        w.flush()?;
        Ok(())
    }

    pub fn export_to_file<P: AsRef<Path>>(&self, file_path: P) -> Result<(), ExportError> {
        let mut f = fs::File::create(file_path)?;
        self.export_to_buf(&mut f)
    }
}

#[derive(Serialize)]
pub struct SerialEditor<'e, N: 'e> {
    inner: &'e N,
    node_states: Vec<(&'e NodeId, &'e NodeState)>,
    scrolling: Vec2,
}

impl<'t, N, T, E, ED> Serialize for NodeEditor<'t, N, T, E, ED>
where
    N: Serialize,
    T: 't + Clone,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let ser = SerialEditor {
            inner: &self.inner,
            node_states: self.node_states.iter().collect(),
            scrolling: self.scrolling.get_current(),
        };
        ser.serialize(serializer)
    }
}

impl<'t, T, E> Serialize for DstEditor<'t, T, E>
where
    T: 't + Clone + Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.dst.serialize(serializer)
    }
}

#[derive(Deserialize)]
#[serde(bound(deserialize = "DN: Deserialize<'de>"))]
pub struct DeserEditor<DN> {
    inner: DN,
    node_states: Vec<(NodeId, NodeState)>,
    scrolling: Vec2,
}

impl<'t, N, T, E, ED> NodeEditor<'t, N, T, E, ED>
where
    T: Clone,
    N: Importable<ImportError<E>>,
{
    fn import_from_buf<R: io::Read>(&mut self, r: R) -> Result<(), ImportError<E>> {
        let deserialized: DeserEditor<N::Deser> = de::from_reader(r)?;

        // Set Ui node states
        self.node_states = {
            let mut node_states = NodeStates::new();
            for (node_id, state) in deserialized.node_states {
                node_states.insert(node_id, state);
            }
            node_states
        };
        // Set scrolling offset
        self.scrolling = Scrolling::new(deserialized.scrolling);
        self.inner.import(deserialized.inner)?;

        Ok(())
    }

    fn import_from_file<P: AsRef<Path>>(&mut self, file_path: P) -> Result<(), ImportError<E>> {
        let f = fs::File::open(file_path)?;
        self.import_from_buf(f)
    }
}

pub trait Importable<Err>: Sized {
    type Deser: for<'de> serde::Deserialize<'de>;

    fn import(&mut self, Self::Deser) -> Result<(), Err>;
}

impl<'t, T, E> Importable<ImportError<E>> for DstEditor<'t, T, E>
where
    T: 'static + Clone + for<'de> Deserialize<'de> + cake::NamedAlgorithms<E> + cake::VariantName,
    E: 'static,
{
    type Deser = DeserEditor<DeserDST<T, E>>;

    fn import(&mut self, import: DeserEditor<DeserDST<T, E>>) -> Result<(), ImportError<E>> {
        // Replace DST. Wait for no computing to take place.
        use std::{thread, time};
        const SLEEP_INTERVAL_MS: u64 = 1;
        let sleep_interval = time::Duration::from_millis(SLEEP_INTERVAL_MS);
        println!("Import requested! Wait for pending compute tasks to complete...");
        let now = time::Instant::now();
        loop {
            if !self.is_compute_running() {
                println!("Starting import after {:?}", now.elapsed());
                break;
            } else {
                thread::sleep(sleep_interval);
            }
        }

        self.dst = import.inner.into()?;

        // Reset cache
        self.output_results = {
            let mut output_results = BTreeMap::new();
            for (output_id, _) in self.dst.outputs_iter() {
                output_results.insert(*output_id, compute::new_compute_result());
            }
            output_results
        };
        Ok(())
    }
}

impl<'t, T, E> DstEditor<'t, T, E>
where
    T: Clone,
{
    pub fn is_compute_running(&self) -> bool {
        self.output_results
            .values()
            .any(|result| result.lock().unwrap().is_running())
    }
}

impl<'t, T: 'static, E: 'static> DstEditor<'t, T, E>
where
    T: Clone + cake::VariantName + Send + Sync,
    E: Send + From<MacroEvaluationError<E>>,
{
    /// Compute output's result asynchonously.
    ///
    /// `self` should live longer as long as computing is not finished.
    /// If not, you'll get undefined behavior!
    pub unsafe fn compute_output(&self, id: cake::OutputId) -> ComputeResult<T, E> {
        let result_lock = &self.output_results[&id];
        let mut result = result_lock.lock().unwrap();
        if result.is_running() {
            // Currently computing... Nothing to do
            drop(result);
        } else {
            result.set_running();
            drop(result);
            let result_lock_clone = result_lock.clone();
            // Extend dst's lifetime
            let dst: &'static DST<T, E> = mem::transmute(&self.dst);
            rayon::spawn(move || {
                let result = dst.compute(id);
                result_lock_clone.lock().unwrap().complete(result);
            });
        }
        result_lock.clone()
    }
}

use constant_editor::ConstantEditor;
use id_stack::GetId;
use imgui::{
    sys, ImGuiCol, ImGuiKey, ImGuiMouseCursor, ImGuiSelectableFlags, ImMouseButton, ImString,
    ImVec2, StyleVar, Ui, WindowDrawList,
};

const NODE_FRAME_COLOR: [f32; 3] = [0.39, 0.39, 0.39];
const NODE_WINDOW_PADDING: Vec2 = Vec2(5.0, 5.0);
const CURRENT_FONT_WINDOW_SCALE: f32 = 1.0;

impl<'t, N, T, E, ED> NodeEditor<'t, N, T, E, ED>
where
    N: for<'a> NodeEditable<'a, 't, T, E> + Importable<ImportError<E>> + Serialize,
    T: 'static
        + Clone
        + cake::EditableVariants
        + cake::NamedAlgorithms<E>
        + cake::VariantName
        + cake::DefaultFor
        + Serialize
        + for<'de> Deserialize<'de>,
    ED: ConstantEditor<T>,
    E: 'static + error::Error,
{
    pub fn render(&mut self, ui: &Ui) {
        for idx in self.inner.dst().node_ids() {
            // Initialization of node states
            self.node_states.init_node(&idx);
        }

        if self.show_left_pane {
            self.render_left_pane(ui);
        }
        self.render_graph_node(ui);

        // Render error popup
        if !self.error_stack.is_empty() {
            ui.open_popup(im_str!("Error!"));
        }
        ui.popup_modal(im_str!("Error!")).build(|| {
            unsafe {
                sys::igPushTextWrapPos(400.0);
            }
            {
                let e = &self.error_stack[self.error_stack.len() - 1];
                ui.text_wrapped(&ImString::new(format!("{}", e)));
            }
            unsafe {
                sys::igPopTextWrapPos();
            }
            if !ui.is_window_hovered() && ui.imgui().is_mouse_clicked(ImMouseButton::Left) {
                self.error_stack.pop();
                ui.close_current_popup();
            }
        });

        if ui.is_window_focused() {
            let delete_index = ui.imgui().get_key_index(ImGuiKey::Delete);
            if ui.imgui().is_key_down(delete_index) {
                self.delete_selected_nodes();
            }
        }
        self.scrolling.tick();
    }

    pub fn outputs(&self) -> Vec<cake::OutputId> {
        self.inner
            .dst()
            .outputs_iter()
            .filter(|(_, some_output)| some_output.is_some())
            .map(|(id, _)| *id)
            .collect()
    }

    fn render_left_pane(&mut self, ui: &Ui) {
        const LEFT_PANE_DEFAULT_RELATIVE_WIDTH: f32 = 0.2;
        let window_size = Vec2::new(ui.get_window_size());
        let pane_width = *self
            .left_pane_size
            .get_or_insert_with(|| window_size.0 * LEFT_PANE_DEFAULT_RELATIVE_WIDTH);

        ui.child_frame(im_str!("node_list"), (pane_width, 0.0))
            .build(|| {
                ui.spacing();
                ui.separator();
                if ui
                    .collapsing_header(im_str!("Node List##node_list_1"))
                    .default_open(true)
                    .build()
                {
                    ui.separator();
                    self.show_node_list(ui);
                }
                ui.separator();
                if let Some(node_id) = self.active_node {
                    ui.spacing();
                    ui.separator();
                    if ui
                        .collapsing_header(im_str!("Active Node##activeNode"))
                        .default_open(true)
                        .build()
                    {
                        ui.separator();
                        let dst = self.inner.dst();
                        let node = dst.get_node(&node_id).unwrap();
                        match node {
                            cake::Node::Transform(t) => {
                                ui.text_wrapped(&ImString::new(format!(
                                    "#{} {}:\n{}",
                                    node_id.id(),
                                    t.name,
                                    t.description
                                )));
                            }
                            cake::Node::Output(_) => {
                                ui.text(format!("#{} Output", -node_id.id()));
                            }
                        }
                    }
                    ui.separator();
                }
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
        const SCROLL_OVER_NODE_OFFSET: Vec2 = Vec2(-50.0, -50.0);

        for (idx, node) in self.inner.dst().nodes_iter() {
            ui.push_id(idx.id());
            let selected = self.node_states.get_state(&idx, |state| state.selected);
            let name = ImString::new(node.name(&idx));
            if ui.selectable(&name, selected, ImGuiSelectableFlags::empty(), (0.0, 0.0)) {
                if !ui.imgui().key_ctrl() {
                    self.node_states.deselect_all();
                }
                self.node_states.toggle_select(&idx);
                self.active_node = Some(idx);
                self.scrolling.set_target(
                    self.node_states.get_state(&idx, |s| s.pos) + SCROLL_OVER_NODE_OFFSET,
                )
            }
            ui.pop_id();
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
                        ui.text(im_str!("Scroll with Ctrl+LMB or Alt+LMB."));
                        ui.same_line(ui.get_window_size().0 - 240.0);
                        if ui.button(im_str!("Import"), (0.0, 0.0)) {
                            if let Err(e) = self.import_from_file("editor_graph_export.ron") {
                                eprintln!("Error on export! {}", e);
                                self.error_stack.push(Box::new(e));
                            }
                        }
                        ui.same_line(ui.get_window_size().0 - 180.0);
                        if ui.button(im_str!("Export"), (0.0, 0.0)) {
                            if let Err(e) = self.export_to_file("editor_graph_export.ron") {
                                eprintln!("Error on import! {}", e);
                                self.error_stack.push(Box::new(e));
                            }
                        }
                        ui.same_line(ui.get_window_size().0 - 120.0);
                        ui.checkbox(im_str!("Show grid"), &mut self.show_grid);
                        ui.text(im_str!("Double-click LMB on slots to remove their links."));
                    });
                }
                const GRAPH_STYLE_VAR: [StyleVar; 2] = [
                    StyleVar::FramePadding(ImVec2 { x: 1.0, y: 1.0 }),
                    StyleVar::WindowPadding(ImVec2 { x: 0.0, y: 0.0 }),
                ];
                const GRAPH_STYLE_COLOR: [(ImGuiCol, (f32, f32, f32, f32)); 1] =
                    [(ImGuiCol::ChildBg, (0.24, 0.24, 0.27, 0.78))];
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
        const NODE_SLOT_RADIUS: f32 = 5.0 * CURRENT_FONT_WINDOW_SCALE;
        const NODE_SLOT_RADIUS_SQUARED: f32 = NODE_SLOT_RADIUS * NODE_SLOT_RADIUS;
        // We don't detect "mouse release" events while dragging links onto slots.
        // Instead we check that our mouse delta is small enough. Otherwise we couldn't
        // hover other slots while dragging links.
        const BASE_NODE_WIDTH: f32 = 120.0 * CURRENT_FONT_WINDOW_SCALE;
        ui.with_item_width(BASE_NODE_WIDTH, || {
            let draw_list = ui.get_window_draw_list();
            draw_list.channels_split(5, |channels| {
                let canvas_size = Vec2::new(ui.get_window_size());
                let win_pos = Vec2::new(ui.get_cursor_screen_pos());
                // TODO: Center view on a specific node
                let offset = win_pos - self.scrolling.get_current();

                if self.show_grid {
                    let cursor_pos = Vec2::new(ui.get_cursor_pos());
                    let offset2 = cursor_pos - self.scrolling.get_current();
                    const GRID_COLOR: [f32; 4] = [0.78, 0.78, 0.78, 0.16];
                    const GRID_SIZE: f32 = 64.0;
                    const GRID_LINE_WIDTH: f32 = 1.0;
                    let grid_sz = CURRENT_FONT_WINDOW_SCALE * GRID_SIZE;
                    let grid_line_width = CURRENT_FONT_WINDOW_SCALE * GRID_LINE_WIDTH;
                    let mut x = offset2.0 % grid_sz;
                    while x < canvas_size.0 {
                        let p1 = Vec2::new((x + win_pos.0, win_pos.1));
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
                if ui.is_window_hovered() {
                    // Create new node with a popup
                    if ui.imgui().is_mouse_clicked(ImMouseButton::Right) {
                        let mouse_pos = ui.imgui().mouse_pos();
                        if win_pos.0 < mouse_pos.0
                            && mouse_pos.0 < win_pos.0 + canvas_size.0
                            && win_pos.1 < mouse_pos.1
                            && mouse_pos.1 < win_pos.1 + canvas_size.1
                        {
                            ui.open_popup(im_str!("add-new-node"));
                        }
                    }
                    // Scroll
                    if self.drag_node.is_none()
                        && self.creating_link.is_none()
                        && (ui.imgui().key_ctrl() || ui.imgui().key_alt())
                        && ui.imgui().is_mouse_dragging(ImMouseButton::Left)
                    {
                        ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeAll);
                        let delta = Vec2(0.0, 0.0) - ui.imgui().mouse_delta().into();
                        self.scrolling.set_delta(delta);
                    }
                }

                // Bezier control point of the links
                const LINK_CONTROL_POINT_DISTANCE: f32 = 50.0;
                let link_cp =
                    Vec2::new((LINK_CONTROL_POINT_DISTANCE * CURRENT_FONT_WINDOW_SCALE, 0.0));
                const LINK_LINE_WIDTH: f32 = 3.0;
                let link_line_width = LINK_LINE_WIDTH * CURRENT_FONT_WINDOW_SCALE;
                // NODE LINK CULLING?

                let mut dst = self.inner.dst_mut();
                for idx in dst.node_ids() {
                    let node_pos = self
                        .node_states
                        .get_state(&idx, |state| state.get_pos(CURRENT_FONT_WINDOW_SCALE));
                    ui.push_id(idx.id());

                    // Display node contents first in the foreground
                    channels.set_current(if self.active_node == Some(idx) { 4 } else { 2 });

                    let node_rect_min = offset + node_pos;
                    let node_rect_max = self
                        .node_states
                        .get_state(&idx, |state| node_rect_min + state.size);
                    ui.set_cursor_screen_pos(node_rect_min + NODE_WINDOW_PADDING);
                    Self::draw_node_inside(
                        &mut dst,
                        ui,
                        &draw_list,
                        &idx,
                        &mut self.node_states,
                        &self.constant_editor,
                        &mut self.active_node,
                        &mut self.drag_node,
                    );

                    let node = dst.get_node(&idx).unwrap();
                    let node_states = &mut self.node_states;
                    let item_rect_size = Vec2::new(ui.get_item_rect_size());
                    node_states.set_state(&idx, |state| {
                        state.size = item_rect_size + NODE_WINDOW_PADDING * 2.0;
                    });

                    channels.set_current(if self.active_node == Some(idx) { 3 } else { 1 });
                    ui.set_cursor_screen_pos(node_rect_min);
                    ui.invisible_button(
                        im_str!("node##nodeinvbtn"),
                        node_states.get_state(&idx, |state| state.size),
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
                    let line_thickness = if node_states.get_state(&idx, |s| s.selected) {
                        3.0
                    } else {
                        1.0
                    } * CURRENT_FONT_WINDOW_SCALE;
                    draw_list
                        .add_rect(node_rect_min, node_rect_max, NODE_FRAME_COLOR)
                        .thickness(line_thickness)
                        .rounding(NODE_ROUNDING)
                        .build();

                    // Display connectors
                    const CONNECTOR_BORDER_THICKNESS: f32 = NODE_SLOT_RADIUS * 0.25;
                    const INPUT_SLOT_COLOR: [f32; 4] = [0.59, 0.59, 0.59, 0.59];
                    for (slot_idx, &slot_name) in node.inputs_iter().enumerate() {
                        let connector_pos = Vec2::new(node_states.get_state(&idx, |state| {
                            state.get_input_slot_pos(
                                slot_idx,
                                node.inputs_count(),
                                CURRENT_FONT_WINDOW_SCALE,
                            )
                        }));
                        let connector_screen_pos = offset + connector_pos;
                        draw_list
                            .add_circle(connector_screen_pos, NODE_SLOT_RADIUS, INPUT_SLOT_COLOR)
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
                        if ui.imgui().is_mouse_clicked(ImMouseButton::Left) {
                            let mouse_pos: Vec2 = ui.imgui().mouse_pos().into();
                            if (mouse_pos - connector_screen_pos).squared_norm()
                                <= NODE_SLOT_RADIUS_SQUARED
                            {
                                self.drag_node = None;
                                self.creating_link = Some(LinkExtremity::Input(match idx {
                                    cake::NodeId::Transform(t_idx) => {
                                        InputSlot::Transform(cake::Input::new(t_idx, slot_idx))
                                    }
                                    cake::NodeId::Output(output_id) => InputSlot::Output(output_id),
                                }));
                            }
                        }
                        if let Some(LinkExtremity::Output(link_output)) = self.creating_link {
                            // Check if we hover slot!
                            let mouse_pos: Vec2 = ui.imgui().mouse_pos().into();
                            if (mouse_pos - connector_screen_pos).squared_norm()
                                <= NODE_SLOT_RADIUS_SQUARED
                            {
                                self.new_link = Some((
                                    link_output,
                                    match idx {
                                        cake::NodeId::Transform(t_idx) => {
                                            InputSlot::Transform(cake::Input::new(t_idx, slot_idx))
                                        }
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
                            let connector_pos = node_states.get_state(&idx, |state| {
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
                                ).thickness(CONNECTOR_BORDER_THICKNESS)
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
                            if ui.imgui().is_mouse_clicked(ImMouseButton::Left) {
                                let mouse_pos: Vec2 = ui.imgui().mouse_pos().into();
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
                                let mouse_pos: Vec2 = ui.imgui().mouse_pos().into();
                                if (mouse_pos - connector_screen_pos).squared_norm()
                                    <= NODE_SLOT_RADIUS_SQUARED
                                {
                                    self.new_link =
                                        Some((cake::Output::new(t_idx, slot_idx), link_input));
                                    self.creating_link = None;
                                }
                            }
                        }
                    }
                    ui.pop_id();
                }
                // Preview new link
                const NEW_LINK_COLOR: [f32; 3] = [0.78, 0.78, 0.39];
                if let Some(ref creating_link) = self.creating_link {
                    if ui.imgui().is_mouse_dragging(ImMouseButton::Left) {
                        let (p1, cp1, cp2, p2) = match *creating_link {
                            LinkExtremity::Output(output) => {
                                let output_node_count =
                                    dst.get_transform(output.t_idx).unwrap().output.len();
                                let output_node_state = self
                                    .node_states
                                    .get(&cake::NodeId::Transform(output.t_idx))
                                    .unwrap();
                                let connector_pos = output_node_state.get_output_slot_pos(
                                    output.index(),
                                    output_node_count,
                                    CURRENT_FONT_WINDOW_SCALE,
                                );
                                let p1 = offset + connector_pos;
                                let p2: Vec2 = ui.imgui().mouse_pos().into();
                                let cp1 = p1 + link_cp;
                                let cp2 = p2 - link_cp;
                                (p1, cp1, cp2, p2)
                            }
                            LinkExtremity::Input(input_slot) => {
                                let connector_pos = match input_slot {
                                    InputSlot::Transform(input) => {
                                        let input_node_count =
                                            dst.get_transform(input.t_idx).unwrap().input.len();
                                        let input_node_state = self
                                            .node_states
                                            .get(&cake::NodeId::Transform(input.t_idx))
                                            .unwrap();
                                        input_node_state.get_input_slot_pos(
                                            input.index(),
                                            input_node_count,
                                            CURRENT_FONT_WINDOW_SCALE,
                                        )
                                    }
                                    InputSlot::Output(output_id) => {
                                        let input_node_state = self
                                            .node_states
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
                                let p2: Vec2 = ui.imgui().mouse_pos().into();
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
                }
                if self.creating_link.is_some() && !ui.imgui().is_mouse_down(ImMouseButton::Left) {
                    self.creating_link = None;
                }

                // Display links
                channels.set_current(0);
                for (output, input_slot) in dst.links_iter() {
                    let connector_in_pos = match input_slot {
                        cake::InputSlot::Transform(input) => {
                            let input_node_count =
                                dst.get_transform(input.t_idx).unwrap().input.len();
                            let input_node_state = self
                                .node_states
                                .get(&cake::NodeId::Transform(input.t_idx))
                                .unwrap();
                            input_node_state.get_input_slot_pos(
                                input.index(),
                                input_node_count,
                                CURRENT_FONT_WINDOW_SCALE,
                            )
                        }
                        cake::InputSlot::Output(output_id) => {
                            let input_node_state = self
                                .node_states
                                .get(&cake::NodeId::Output(output_id))
                                .unwrap();
                            input_node_state.get_input_slot_pos(
                                0usize,
                                1usize,
                                CURRENT_FONT_WINDOW_SCALE,
                            )
                        }
                    };
                    let p1 = offset + connector_in_pos;
                    let output_node_count = dst.get_transform(output.t_idx).unwrap().output.len();
                    let output_node_state = self
                        .node_states
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
        if let Some((output, input_slot)) = self.new_link {
            match input_slot {
                InputSlot::Transform(input) => {
                    if let Err(e) = self.inner.dst_mut().connect(output, input) {
                        eprintln!("{:?}", e);
                        self.error_stack.push(Box::new(e));
                    }
                }
                InputSlot::Output(output_id) => {
                    self.inner.dst_mut().update_output(output_id, output)
                }
            }
            self.new_link = None;
        }
        ui.popup(im_str!("add-new-node"), || {
            ui.text("Add node");
            ui.separator();
            for (i, node) in self.addable_nodes.iter().enumerate() {
                ui.push_id(i as i32);
                if ui.menu_item(&ImString::new(node.name)).build() {
                    self.inner.dst_mut().add_transform(node);
                }
                ui.pop_id();
            }
            ui.separator();
            if ui.menu_item(im_str!("Output node")).build() {
                let id = self.inner.dst_mut().create_output();
                // TODO: Update ouputs!
                // self.output_results
                //     .insert(id, compute::new_compute_result());
            }
            ui.separator();
            for constant_type in T::editable_variants() {
                let item_name = ImString::new(format!("Input node: {}", constant_type));
                if ui.menu_item(&item_name).build() {
                    let constant = Transformation::new_constant(T::default_for(constant_type));
                    self.inner.dst_mut().add_owned_transform(constant);
                }
            }
        });
    }

    fn draw_node_inside<D: DerefMut<Target = DST<'t, T, E>>>(
        dst: &mut D,
        ui: &Ui,
        draw_list: &WindowDrawList,
        id: &cake::NodeId,
        node_states: &mut NodeStates,
        constant_editor: &ED,
        active_node: &mut Option<NodeId>,
        drag_node: &mut Option<NodeId>,
    ) {
        let mut dst = dst.deref_mut();
        let node_name = {
            let node = dst.get_node(id).unwrap();
            ImString::new(node.name(id))
        };
        let mut title_bar_height = 0.0;
        let p = ui.get_cursor_screen_pos();

        ui.group(|| {
            let default_text_color = ui.imgui().style().colors[ImGuiCol::Text as usize];
            ui.with_color_var(ImGuiCol::Text, default_text_color, || {
                ui.text(&node_name);
                title_bar_height = ui.get_item_rect_size().1;
                if ui.is_item_hovered() {
                    ui.tooltip(|| {
                        ui.text(format!(
                            "Node #{}: {:?}",
                            match id {
                                cake::NodeId::Output(_) => -id.id(),
                                cake::NodeId::Transform(_) => id.id(),
                            },
                            &node_name
                        ))
                    });
                }
            });
            ui.dummy((0.0, NODE_WINDOW_PADDING.1 / 2.0));
            let mut purge_list = Vec::new();
            if let cake::NodeId::Transform(t_idx) = *id {
                if let Some(t) = dst.get_transform_mut(t_idx) {
                    if let cake::Algorithm::Constant(ref mut constants) = t.algorithm {
                        for c in constants.iter_mut() {
                            if constant_editor.editor(ui, c) {
                                purge_list.push(id);
                            }
                        }
                    }
                }
                if let Some(default_inputs) = dst.get_default_inputs_mut(t_idx) {
                    for default_input in default_inputs.iter_mut() {
                        if let Some(default_input) = default_input {
                            if constant_editor.editor(ui, default_input) {
                                purge_list.push(id);
                            }
                        } else {
                            // Fill with dummy line for vertical alignment
                            ui.text("");
                        }
                    }
                }
            }
            for node_id in purge_list {
                dst.purge_cache_node(node_id);
            }
            // TODO: Add copy-paste buttons
        });

        // Line below node name
        let node_size = ui.get_item_rect_size();
        let line_thickness =
            if *active_node == Some(*id) { 3.0 } else { 1.0 } * CURRENT_FONT_WINDOW_SCALE;
        draw_list
            .add_line(
                [
                    p.0 - NODE_WINDOW_PADDING.0,
                    p.1 + title_bar_height + NODE_WINDOW_PADDING.1 / 2.0,
                ],
                [
                    p.0 + node_size.0 + NODE_WINDOW_PADDING.0,
                    p.1 + title_bar_height + NODE_WINDOW_PADDING.1 / 2.0,
                ],
                NODE_FRAME_COLOR,
            ).thickness(line_thickness)
            .build();

        if ui.is_item_hovered() && ui.imgui().is_mouse_clicked(ImMouseButton::Left) {
            *active_node = Some(*id);
            *drag_node = Some(*id);
            if !ui.imgui().key_ctrl() {
                node_states.deselect_all();
            }
            node_states.toggle_select(id);
        }
        if *drag_node == Some(*id) {
            if ui.imgui().is_mouse_dragging(ImMouseButton::Left) {
                let delta = ui.imgui().mouse_delta();
                node_states.set_state(id, |state| {
                    state.pos = state.pos + delta.into();
                });
            } else if !ui.imgui().is_mouse_down(ImMouseButton::Left) {
                *drag_node = None;
            }
        }
    }
}

impl<'t, N, T, E, ED> NodeEditor<'t, N, T, E, ED>
where
    N: for<'a> NodeEditable<'a, 't, T, E>,
    T: Clone,
{
    fn delete_selected_nodes(&mut self) {
        let selected_node_ids: Vec<_> = self
            .node_states
            .iter()
            .filter(|(_, state)| state.selected)
            .map(|(id, _)| *id)
            .collect();
        for node_id in selected_node_ids {
            self.inner.dst_mut().remove_node(&node_id);
            self.node_states.remove_node(&node_id);
            if self.active_node == Some(node_id) {
                self.active_node.take();
            }
        }
    }
}