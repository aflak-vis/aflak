use std::collections::BTreeMap;

use cake;

use vec2::Vec2;

#[derive(Debug)]
pub struct NodeState {
    pub selected: bool,
    pub open: bool,
    pub pos: Vec2,
    pub size: Vec2,
}

impl Default for NodeState {
    fn default() -> Self {
        Self {
            selected: false,
            open: true,
            pos: Vec2::default(),
            size: Vec2::default(),
        }
    }
}

impl NodeState {
    pub fn get_pos(&self, font_window_scale: f32) -> Vec2 {
        self.pos * font_window_scale
    }

    pub fn get_input_slot_pos<I: Into<usize>, C: Into<usize>>(
        &self,
        slot_idx: I,
        slot_cnt: C,
        font_window_scale: f32,
    ) -> Vec2 {
        Vec2::new((
            self.pos.0 * font_window_scale,
            self.pos.1 * font_window_scale
                + self.size.1 * (slot_idx.into() + 1) as f32 / (slot_cnt.into() + 1) as f32,
        ))
    }

    pub fn get_output_slot_pos<I: Into<usize>, C: Into<usize>>(
        &self,
        slot_idx: I,
        slot_cnt: C,
        font_window_scale: f32,
    ) -> Vec2 {
        Vec2::new((
            self.pos.0 * font_window_scale + self.size.0,
            self.pos.1 * font_window_scale
                + self.size.1 * (slot_idx.into() + 1) as f32 / (slot_cnt.into() + 1) as f32,
        ))
    }
}

pub type NodeStates = BTreeMap<cake::NodeId, NodeState>;

pub fn deselect_all_nodes(node_states: &mut NodeStates) {
    for state in node_states.values_mut() {
        state.selected = false;
    }
}

pub fn toggle_select_node(node_states: &mut NodeStates, id: &cake::NodeId) {
    let state = node_states.get_mut(id).unwrap();
    state.selected = !state.selected;
}

pub fn open_node(node_states: &mut NodeStates, idx: &cake::NodeId, open: bool) {
    let state = node_states.get_mut(idx).unwrap();
    state.open = open;
}

pub fn node_state_get<T, F: FnOnce(&NodeState) -> T>(
    node_states: &NodeStates,
    id: &cake::NodeId,
    f: F,
) -> T {
    let state = node_states.get(id).unwrap();
    f(state)
}

pub fn node_state_set<T, F: FnOnce(&mut NodeState) -> T>(
    node_states: &mut NodeStates,
    id: &cake::NodeId,
    f: F,
) -> T {
    let state = node_states.get_mut(id).unwrap();
    f(state)
}

pub fn init_node(node_states: &NodeStates) -> NodeState {
    let mut max = -300.0;
    for state in node_states.values() {
        if state.pos.1 > max {
            max = state.pos.1;
        }
    }
    NodeState {
        pos: Vec2::new((0.0, max + 150.0)),
        ..Default::default()
    }
}
