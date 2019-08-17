//! Draw 3D representations.

use std::borrow::Cow;

use glium::{
    backend::Facade,
    texture::{ClientFormat, RawImage2d},
    Texture2d,
};
use imgui::{ImTexture, Ui};
use ndarray::{ArrayBase, Data, IxDyn};

use super::imshow::Textures;

/// TODO
pub trait UiImage3d {
    fn image3d<S, F>(
        &self,
        image: &ArrayBase<S, IxDyn>,
        texture_id: ImTexture,
        textures: &mut Textures,
        ctx: &F,
    ) where
        S: Data<Elem = f32>,
        F: Facade;
}

impl<'ui> UiImage3d for Ui<'ui> {
    // TODO
    fn image3d<S, F>(
        &self,
        image: &ArrayBase<S, IxDyn>,
        texture_id: ImTexture,
        textures: &mut Textures,
        ctx: &F,
    ) where
        S: Data<Elem = f32>,
        F: Facade,
    {
        let p = self.get_cursor_screen_pos();
        let window_pos = self.get_window_pos();
        let window_size = self.get_window_size();
        let size = (window_size.0, window_size.1 - (p.1 - window_pos.1));

        // 3D image...
        let raw = make_raw_image(image);
        let gl_texture = Texture2d::new(ctx, raw).expect("Error!");
        textures.replace(texture_id, gl_texture);

        self.image(texture_id, size).build();
    }
}

fn make_raw_image<S>(image: &ArrayBase<S, IxDyn>) -> RawImage2d<'static, u8>
where
    S: Data<Elem = f32>,
{
    let image3 = image.slice(s![.., .., ..]);
    let n = 10;
    let m = 10;
    let data = vec![255; 3 * n * m];
    RawImage2d {
        data: Cow::Owned(data),
        width: n as u32,
        height: m as u32,
        format: ClientFormat::U8U8U8,
    }
}
