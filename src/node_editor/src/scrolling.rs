use vec2::Vec2;

#[derive(Default)]
pub struct Scrolling {
    target: Vec2,
}

impl Scrolling {
    pub fn new(target: Vec2) -> Scrolling {
        Scrolling { target }
    }

    pub fn get_current(&self) -> Vec2 {
        self.target
    }

    pub fn set_target(&mut self, target: Vec2) {
        self.target = target;
    }

    pub fn set_delta(&mut self, delta: Vec2) {
        self.target = self.target + delta;
    }
}
