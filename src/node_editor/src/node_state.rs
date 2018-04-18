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
