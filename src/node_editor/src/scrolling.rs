use vec2::Vec2;

#[derive(Default)]
pub struct Scrolling {
    target: Vec2,
    current: Vec2,
}

impl Scrolling {
    pub fn new(target: Vec2) -> Scrolling {
        Scrolling {
            target,
            current: target,
        }
    }

    pub fn get_current(&self) -> Vec2 {
        self.current
    }

    /// Set target fluidly
    pub fn set_target(&mut self, target: Vec2) {
        self.target = target;
    }

    pub fn set_delta(&mut self, delta: Vec2) {
        self.target = self.target + delta;
        self.current = self.target;
    }

    /// Get current closer to target
    pub fn tick(&mut self) {
        const SPEED: f32 = 0.1;

        let diff = self.target - self.current;
        self.current = self.current + diff * SPEED;
    }
}
