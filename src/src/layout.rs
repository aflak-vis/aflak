use std::f32::NAN;

use imgui::ImString;

pub struct LayoutEngine {
    outputs: Vec<ImString>,
}

const EDITOR_WINDOW_DEFAULT_POSITION: (f32, f32) = (50.0, 50.0);
const EDITOR_WINDOW_DEFAULT_SIZE: (f32, f32) = (1200.0, 800.0);

const OUTPUT_WINDOW_DEFAULT_SIZE: (f32, f32) = (600.0, 400.0);
const OUTPUT_WINDOW_DEFAULT_MARGIN: f32 = 50.0;

impl LayoutEngine {
    pub fn new() -> LayoutEngine {
        LayoutEngine { outputs: vec![] }
    }

    pub fn default_editor_window_position(&self) -> (f32, f32) {
        EDITOR_WINDOW_DEFAULT_POSITION
    }

    pub fn default_editor_window_size(&self) -> (f32, f32) {
        EDITOR_WINDOW_DEFAULT_SIZE
    }

    /// Align windows on a 4-column-wide grid
    pub fn default_output_window_position_size(
        &mut self,
        name: &ImString,
    ) -> ((f32, f32), (f32, f32)) {
        if self.outputs.contains(name) {
            ((NAN, NAN), (NAN, NAN))
        } else {
            let (row, col) = {
                let n = self.outputs.len();
                let row = n / 4;
                let col = n % 4;
                (row as f32, col as f32)
            };
            self.outputs.push(name.clone());
            (
                (
                    EDITOR_WINDOW_DEFAULT_POSITION.0
                        + col * (OUTPUT_WINDOW_DEFAULT_SIZE.0 + OUTPUT_WINDOW_DEFAULT_MARGIN),
                    EDITOR_WINDOW_DEFAULT_POSITION.1
                        + EDITOR_WINDOW_DEFAULT_SIZE.1
                        + OUTPUT_WINDOW_DEFAULT_MARGIN
                        + row * (OUTPUT_WINDOW_DEFAULT_SIZE.1 + OUTPUT_WINDOW_DEFAULT_MARGIN),
                ),
                OUTPUT_WINDOW_DEFAULT_SIZE,
            )
        }
    }
}
