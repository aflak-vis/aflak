use glium::{backend::Facade, Texture2d};
use std::time::Instant;

pub struct Image {
    created_on: Option<Instant>,
}

impl Default for Image {
    fn default() -> Self {
        Self { created_on: None }
    }
}

impl Image {
    pub fn created_on(&self) -> Option<Instant> {
        self.created_on
    }
}

#[derive(Copy, Clone)]
pub struct Vertex {
    pos: [f32; 3],
    texcoord: [f32; 2],
}
implement_vertex!(Vertex, pos, texcoord);

pub struct VolumeContext<'a> {
    //pub eventsLoop: glium::glutin::EventsLoop,
    //pub wb: glium::glutin::WindowBuilder,
    //pub cb: glium::glutin::ContextBuilder<'a>,
    //pub display: glium::Display,
    pub program: glium::Program,
    pub fb_tex: glium::Texture2d,
    pub fb: glium::framebuffer::SimpleFrameBuffer<'a>,
    pub vb: glium::VertexBuffer<Vertex>,
    pub ib: glium::index::NoIndices,
    //pub shape: (usize, usize, usize),
    //pub volume_data: std::vec::Vec<std::vec::Vec<std::vec::Vec<f32>>>,
    //pub min_val: f32,
    //pub max_val: f32,
    //pub texture: glium::texture::CompressedTexture3d,
    pub uniforms: glium::uniforms::UniformsStorage<
        'a,
        glium::uniforms::Sampler<'a, glium::texture::CompressedTexture3d>,
        glium::uniforms::EmptyUniforms,
    >,
}

impl Default for VolumeContext<'_> {
    fn default() -> Self {
        let eventsLoop = glium::glutin::EventsLoop::new();
        let wb = glium::glutin::WindowBuilder::new().with_visibility(false);
        let cb = glium::glutin::ContextBuilder::new();
        let display = glium::Display::new(wb, cb, &eventsLoop).unwrap();
        let program = glium::Program::from_source(
                display.get_context(),
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
                    const vec3 eye = vec3(0.0f, 0.0f, -4.0f);
                    const vec3 position_screen =
                    {
                        Texcoord.x * 2.0f - 1.0f,
                        Texcoord.y * 2.0f - 1.0f,
                        eye.z + 1.732f,
                    };
                    const float theta = PI / 4;
                    const float phi = 0.0;
                    const mat3 M1 = mat3(
                        cos(theta), 0, sin(theta),
                        0, 1, 0,
                        -sin(theta), 0, cos(theta)
                    );
                    const mat3 M2 = mat3(
                        1, 0, 0,
                        0, cos(phi), -sin(phi),
                        0, sin(phi), cos(phi)
                    );

                    ray r =
                    {
                        M1 * M2 * eye,
                        M1 * M2 * normalize(position_screen - eye),
                        vec4(0.0f),
                    };

                    sampling_volume(r);

                    out_color = gammaCorrect(r.color, 2.2);
                }",
                None,
            )
            .unwrap();
        let fb_tex = Texture2d::empty_with_format(
            display.get_context(),
            glium::texture::UncompressedFloatFormat::F32F32F32F32,
            glium::texture::MipmapsOption::NoMipmap,
            1024 as u32,
            1024 as u32,
        )
        .unwrap();
        let fb =
            glium::framebuffer::SimpleFrameBuffer::new(display.get_context(), &fb_tex).unwrap();
        let vb = glium::VertexBuffer::new(
            display.get_context(),
            &[
                Vertex {
                    pos: [1.0, -1.0, 0.0],
                    texcoord: [1.0, 1.0],
                },
                Vertex {
                    pos: [-1.0, -1.0, 0.0],
                    texcoord: [0.0, 1.0],
                },
                Vertex {
                    pos: [-1.0, 1.0, 0.0],
                    texcoord: [0.0, 0.0],
                },
                Vertex {
                    pos: [1.0, 1.0, 0.0],
                    texcoord: [1.0, 0.0],
                },
            ],
        )
        .unwrap();
        let ib = glium::index::NoIndices(glium::index::PrimitiveType::TriangleFan);
        let shape = (76, 76, 76);
        let volume_data = vec![vec![vec![0f32; shape.2]; shape.1]; shape.0];
        let min_val = std::f32::MAX;
        let max_val = 0f32;
        let texture = glium::texture::CompressedTexture3d::new(
            display.get_context(),
            glium::texture::Texture3dDataSource::into_raw(volume_data),
        )
        .unwrap();
        let uniforms = uniform! {volume: texture.sampled().magnify_filter(glium::uniforms::MagnifySamplerFilter::Linear)};
        Self {
            //eventsLoop,
            //wb,
            //cb,
            //display,
            program,
            fb_tex,
            fb,
            vb,
            ib,
            //shape,
            //volume_data,
            //min_val,
            //max_val,
            //texture,
            uniforms,
        }
    }
}

pub struct State {
    pub isInitialized: bool,
    pub vctx: VolumeContext<'static>,
    image: self::Image,
}

impl Default for State {
    fn default() -> Self {
        State {
            isInitialized: false,
            image: Default::default(),
            vctx: Default::default(),
        }
    }
}

impl State {
    pub fn image_created_on(&self) -> Option<Instant> {
        self.image.created_on()
    }
    pub fn set_image(&mut self, created_on: Instant) -> () {
        self.image.created_on = Some(created_on);
    }
}
