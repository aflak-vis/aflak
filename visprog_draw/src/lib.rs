extern crate aflak_cake as cake;
extern crate glium;

use glium::{DrawError, Surface};

/// Draw options to be provided to the draw function
pub struct DrawOptions {
    clear_color: [f32; 4],
}

impl Default for DrawOptions {
    fn default() -> Self {
        Self {
            clear_color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

/// Draw the DST to the given target
pub fn draw<'t, S, T, E>(
    target: &mut S,
    dst: &cake::DST<'t, T, E>,
    options: &DrawOptions,
) -> Result<(), DrawError>
where
    S: Surface,
    T: Clone,
{
    let [r, g, b, a] = options.clear_color;
    target.clear_color(r, g, b, a);

    Ok(())
}
