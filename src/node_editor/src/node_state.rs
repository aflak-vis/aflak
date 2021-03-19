use std::collections::{btree_map, BTreeMap};

use crate::cake;

use crate::vec2::Vec2;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeState {
    #[serde(skip_serializing, skip_deserializing)]
    pub selected: bool,
    pub pos: Vec2,
    pub size: Vec2,
}

impl Default for NodeState {
    fn default() -> Self {
        Self {
            selected: false,
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

#[derive(Clone, Debug)]
pub struct NodeStates(BTreeMap<cake::NodeId, NodeState>);

impl NodeStates {
    pub fn get(&self, id: &cake::NodeId) -> Option<&NodeState> {
        self.0.get(id)
    }

    pub fn iter(&self) -> btree_map::Iter<cake::NodeId, NodeState> {
        self.0.iter()
    }

    /// Insert/Replace state for specific [`cake::NodeId`].
    /// Only used to reconstruct [`NodeStates`] during import.
    pub fn insert(&mut self, id: cake::NodeId, state: NodeState) {
        self.0.insert(id, state);
    }

    pub fn new() -> Self {
        NodeStates(BTreeMap::new())
    }

    pub fn init_node(&mut self, id: &cake::NodeId, clue: Vec2) {
        if !self.0.contains_key(id) {
            let new_node = init_node(self, clue);
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

    pub fn get_state<T, F>(&self, id: &cake::NodeId, f: F) -> T
    where
        F: FnOnce(&NodeState) -> T,
    {
        let state = &self.0[id];
        f(state)
    }

    pub fn set_state<T, F>(&mut self, id: &cake::NodeId, f: F) -> T
    where
        F: FnOnce(&mut NodeState) -> T,
    {
        let state = self.0.get_mut(id).unwrap();
        f(state)
    }

    pub fn remove_node(&mut self, id: &cake::NodeId) -> Option<NodeState> {
        self.0.remove(id)
    }
}

fn init_node(node_states: &NodeStates, clue: Vec2) -> NodeState {
    let mut pos = clue;
    // Run very simple heuristic to prevent new nodes from appearing on top of a
    // previous node
    for state in node_states.0.values() {
        if pos.0 >= state.pos.0
            && pos.0 <= state.pos.0 + state.size.0
            && pos.1 >= state.pos.1
            && pos.1 <= state.pos.1 + state.size.1
        {
            // pos is over another node, so move it aside (on the right)
            const MIN_MARGIN_BETWEEN_NODES: f32 = 10.0;
            pos.0 = state.pos.0 + state.size.0 + MIN_MARGIN_BETWEEN_NODES;
        }
    }
    NodeState {
        pos,
        ..Default::default()
    }
}
