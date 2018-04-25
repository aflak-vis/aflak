use std::collections::{btree_map, BTreeMap};

use cake;

use vec2::Vec2;

#[derive(Debug, Serialize)]
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

pub struct NodeStates(BTreeMap<cake::NodeId, NodeState>);

impl NodeStates {
    pub fn get(&self, id: &cake::NodeId) -> Option<&NodeState> {
        self.0.get(id)
    }

    pub fn iter(&self) -> btree_map::Iter<cake::NodeId, NodeState> {
        self.0.iter()
    }

    pub fn new() -> Self {
        NodeStates(BTreeMap::new())
    }

    pub fn init_node(&mut self, id: &cake::NodeId) {
        if !self.0.contains_key(id) {
            let new_node = init_node(self);
            self.0.insert(*id, new_node);
        }
    }

    pub fn deselect_all(&mut self) {
        for state in self.0.values_mut() {
            state.selected = false;
        }
    }

    pub fn toggle_select(&mut self, id: &cake::NodeId) {
        let state = self.0.get_mut(id).unwrap();
        state.selected = !state.selected;
    }

    pub fn open_node(&mut self, idx: &cake::NodeId, open: bool) {
        let state = self.0.get_mut(idx).unwrap();
        state.open = open;
    }

    pub fn get_state<T, F>(&self, id: &cake::NodeId, f: F) -> T
    where
        F: FnOnce(&NodeState) -> T,
    {
        let state = self.0.get(id).unwrap();
        f(state)
    }

    pub fn set_state<T, F>(&mut self, id: &cake::NodeId, f: F) -> T
    where
        F: FnOnce(&mut NodeState) -> T,
    {
        let state = self.0.get_mut(id).unwrap();
        f(state)
    }
}

fn init_node(node_states: &NodeStates) -> NodeState {
    let mut max = -300.0;
    for state in node_states.0.values() {
        if state.pos.1 > max {
            max = state.pos.1;
        }
    }
    NodeState {
        pos: Vec2::new((0.0, max + 150.0)),
        ..Default::default()
    }
}
