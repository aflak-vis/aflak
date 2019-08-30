//! Draw 3D representations.
use std::borrow::Cow;
use glium::{
    backend::Facade,
    texture::{ClientFormat, RawImage2d},
    Texture2d,
    Surface,
};
use imgui::{ImTexture, Ui};
use ndarray::{ArrayBase, Data, Ix3, IxDyn};

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
        let size = (
            window_size.0 - 15.0,
            window_size.1 - (p.1 - window_pos.1) - 10.0,
        );

        // 3D image...
        let raw = make_raw_image(image);
        let gl_texture = Texture2d::new(ctx, raw).expect("Error!");
        textures.replace(texture_id, gl_texture);

        self.image(texture_id, size).build();
    }
}

#[derive(Copy, Clone)]
struct Vertex {
    pos: [f32; 3],
    texcoord: [f32; 2],
}
implement_vertex!(Vertex, pos, texcoord);

fn ray_casting_gpu<S>(image: &ArrayBase<S, Ix3>, n: usize, m: usize) -> Vec<u8>
where
    S: Data<Elem = f32>,
{
    let mut data = vec![0; 3 * n * m];

    let events_loop = glium::glutin::EventsLoop::new();
    let wb = glium::glutin::WindowBuilder::new()
        .with_visibility(false);
    let cb = glium::glutin::ContextBuilder::new();
    let display = glium::Display::new(wb, cb, &events_loop).unwrap();
    let program = glium::Program::from_source(display.get_context(),
        "#version 430

        in vec3 pos;
        in vec2 texcoord;
        out vec2 Texcoord;
        void main() {
          gl_Position = vec4(pos, 1.0);
          Texcoord = texcoord;
        }",
        "#version 430

        #define PI (3.14159265359)
        #define FLT_MAX (3.402823466e+38)

        in vec2 Texcoord;
        out vec4 out_color;
        uniform sampler3D volume;

        struct ray {
            vec3 origin;
            vec3 direction;
            vec4 color;
        };

        bool hit_volume(inout ray r) {
            const int nDim = 3;
            const vec3 _min = {-1.0f, -1.0f, -1.0f};
            const vec3 _max = {1.0f, 1.0f, 1.0f};
            float tmin = 0.0f;
            float tmax = FLT_MAX;
            float t0;
            float t1;
            for (int i = 0; i < nDim; i++) {
                t0 = min((_min[i] - r.origin[i]) / r.direction[i],
                        (_max[i] - r.origin[i]) / r.direction[i]);
                t1 = max((_min[i] - r.origin[i]) / r.direction[i],
                        (_max[i] - r.origin[i]) / r.direction[i]);
                tmin = max(t0, tmin);
                tmax = min(t1, tmax);
                if (tmax <= tmin) {
                        return false;
                }
            }
            r.origin = r.origin + tmin * r.direction;
            return true;
        }

        vec4 color_legend(const in float val) {
            const float temp = (-cos(4.0f * val * PI) + 1.0f) / 2.0f;
            const vec4 result =
                (val > 1.0f) ? vec4(1.0f, 0.0f, 0.0f, 0.0f) :
                (val > 3.0f / 4.0f) ? vec4(1.0f, temp, 0.0f, 0.0f) * val :
                (val > 2.0f / 4.0f) ? vec4(temp, 1.0f, 0.0f, 0.0f) * val :
                (val > 1.0f / 4.0f) ? vec4(0.0f, 1.0f, temp, 0.0f) * val :
                (val > 0.0f) ? vec4(0.0f, temp, 1.0f, 0.0f) * val : vec4(0.0f, 0.0f, 0.0f, 0.0f);

            return result;
        }

        void sampling_volume(inout ray r) {
            const float dt = 1.0f / 10.0f;
            uint step = 1;
            float val;
            while (hit_volume(r))
            {
                val = texture(volume, r.origin / 2.0f + vec3(0.5)).r;
                r.color += (color_legend(val) - r.color) / step++;
                r.origin += r.direction * dt;
            }
        }

        vec4 gammaCorrect(const in vec4 color, const in float gamma) {
            const float g = 1.0f / gamma;
            const vec4 result =
            {
                pow(color.r, g),
                pow(color.g, g),
                pow(color.b, g),
                1.0f,
            };
            return result;
        }

        void main() {
            const vec3 eye = vec3(0.0f, 0.0f, -2.0f);
            const vec3 position_screen =
            {
                Texcoord.x * 2.0f - 1.0f,
                Texcoord.y * 2.0f - 1.0f,
                eye.z + 0.5f,
            };

            ray r =
            {
                eye,
                normalize(position_screen - eye),
                vec4(0.0f),
            };

            sampling_volume(r);

            out_color = gammaCorrect(r.color, 2.2);
        }",
        None
    ).unwrap();

    let fb_tex = Texture2d::empty_with_format(display.get_context(), glium::texture::UncompressedFloatFormat::F32F32F32F32, glium::texture::MipmapsOption::NoMipmap, n as u32, m as u32).unwrap();

    let mut fb = glium::framebuffer::SimpleFrameBuffer::new(display.get_context(), &fb_tex).unwrap();

    let vertex_buffer = glium::VertexBuffer::new(display.get_context(), &[
        Vertex{pos: [1.0, -1.0, 0.0], texcoord: [1.0, 1.0]},
        Vertex{pos: [-1.0, -1.0, 0.0], texcoord: [0.0, 1.0]},
        Vertex{pos: [-1.0, 1.0, 0.0], texcoord: [0.0, 0.0]},
        Vertex{pos: [1.0, 1.0, 0.0], texcoord: [1.0, 0.0]},
    ]).unwrap();
    let index_buffer = glium::index::NoIndices(glium::index::PrimitiveType::TriangleFan);
    let mut shape = image.dim();
    if shape.0 > 128 {shape.0 = 128;} // max texture size is 2048
    let mut volume_data = vec![vec![vec![0f32; shape.2]; shape.1]; shape.0];
    let mut min_val = std::f32::MAX;
    let mut max_val = 0f32;
    for i in 0..shape.0 {
        for j in 0..shape.1 {
            for k in 0..shape.2 {
                if min_val > image[[i, j, k]] {
                    min_val = if image[[i, j, k]] < 0f32 { 0f32 } else { image[[i, j, k]] };
                }
                if max_val < image[[i, j, k]] {
                    max_val = image[[i, j, k]];
                }
            }
        }
    }
    for i in 0..shape.0 {
        for j in 0..shape.1 {
            for k in 0..shape.2 {
                volume_data[i][j][k] = (if image[[i, j, k]] < 0f32 { 0f32 } else { image[[i, j, k]] } - min_val) / (max_val - min_val);
            }
        }
    }
    let texture = glium::texture::CompressedTexture3d::new(display.get_context(), glium::texture::Texture3dDataSource::into_raw(volume_data)).unwrap();
    let uniforms = uniform!{volume: texture.sampled().magnify_filter(glium::uniforms::MagnifySamplerFilter::Linear)};
    fb.draw(&vertex_buffer, &index_buffer, &program, &uniforms, &Default::default()).unwrap();

    let read_back: Vec<Vec<(u8, u8, u8, u8)>> = fb_tex.read();

    for i in 0..n {
        for j in 0..m {
            data[(i * m + j) * 3 + 0] = read_back[i][j].0;
            data[(i * m + j) * 3 + 1] = read_back[i][j].1;
            data[(i * m + j) * 3 + 2] = read_back[i][j].2;
        }
    }

    data
}

fn make_raw_image<S>(image: &ArrayBase<S, IxDyn>) -> RawImage2d<'static, u8>
where
    S: Data<Elem = f32>,
{
    let image3 = image.slice(s![.., .., ..]);
    let n = 256;
    let m = 256;
    let data = ray_casting_gpu(&image3, n, m);
    RawImage2d {
        data: Cow::Owned(data),
        width: n as u32,
        height: m as u32,
        format: ClientFormat::U8U8U8,
    }
}
