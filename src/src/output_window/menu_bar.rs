use std::error;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::Instant;

use glium;

use imgui::{MenuItem, TextureId, Ui, Window};
use owning_ref::ArcRef;

use aflak_plot::{
    imshow::{Textures, UiImage2d},
    plot::UiImage1d,
    AxisTransform, InteractionIterMut, ValueIter,
};
use cake::OutputId;
use primitives::{
    self,
    fitrs::{Fits, Hdu},
    //ndarray::Dimension,
    IOValue,
    ROI,
};

extern crate gaiku_3d;
extern crate obj_exporter;

use output_window::menu_bar::gaiku_3d::{
    bakers::HeightMapBaker,
    common::{nalgebra::Point3, Baker, Chunk, Mesh},
};

use output_window::menu_bar::obj_exporter::{Geometry, ObjSet, Object, Primitive, Shape, Vertex};

use super::{AflakNodeEditor, EditableValues, OutputWindow};

/// Catch-all object for variables used by output window during render
pub struct OutputWindowCtx<'ui, 'val, 'w, 'tex, 'ed, 'gl, F: 'gl> {
    pub ui: &'ui Ui<'ui>,
    pub output: OutputId,
    pub value: &'val ::std::sync::Arc<IOValue>,
    pub window: &'w mut OutputWindow,
    pub created_on: Instant,
    pub node_editor: &'ed mut AflakNodeEditor,
    pub gl_ctx: &'gl F,
    pub textures: &'tex mut Textures,
}

/// Similar to Visualizable, excepts that the types that implements this trait
/// are more complex.
/// The can be exported to disk and display options can be included in a menu
/// bar.
pub trait MenuBar {
    fn visualize<F>(&self, ctx: OutputWindowCtx<'_, '_, '_, '_, '_, '_, F>)
    where
        F: glium::backend::Facade;

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ExportError>;

    fn save_obj<P: AsRef<Path>>(&self, path: P) -> Result<(), ExportError>;

    const EXTENSION: &'static str;

    fn draw<'ui, F>(
        &self,
        ctx: OutputWindowCtx<'ui, '_, '_, '_, '_, '_, F>,
        window: Window<'_>,
    ) -> Vec<Box<dyn error::Error>>
    where
        F: glium::backend::Facade,
    {
        let mut errors = vec![];
        window.menu_bar(true).build(ctx.ui, || {
            errors = MenuBar::menu_bar(self, ctx.ui, ctx.output, ctx.window);
            MenuBar::visualize(self, ctx);
        });
        errors
    }

    fn file_submenu(&self, _: &Ui, _: &mut OutputWindow, OutputId) {}

    fn file_name(&self, output: OutputId) -> String {
        format!("output-{}.{}", output.id(), Self::EXTENSION)
    }

    fn menu_bar(
        &self,
        ui: &Ui,
        output: OutputId,
        window: &mut OutputWindow,
    ) -> Vec<Box<dyn error::Error>> {
        let mut errors: Vec<Box<dyn error::Error>> = vec![];

        let mut output_saved_success_popup = false;
        ui.menu_bar(|| {
            if let Some(menu) = ui.begin_menu(im_str!("File"), true) {
                if MenuItem::new(im_str!("Save")).build(ui) {
                    let path = self.file_name(output);
                    if let Err(e) = self.save(path) {
                        eprintln!("Error on saving output: '{}'", e);
                        errors.push(Box::new(e));
                    } else {
                        output_saved_success_popup = true;
                    }
                }
                self.file_submenu(ui, window, output);
                menu.end(ui);
            }
        });

        if output_saved_success_popup {
            ui.open_popup(im_str!("Export completed!"));
        }
        ui.popup_modal(im_str!("Export completed!")).build(|| {
            ui.text(format!(
                "File saved with success to '{}'.",
                self.file_name(output)
            ));
            if ui.button(im_str!("Close"), [0.0, 0.0]) {
                ui.close_current_popup();
            }
        });

        errors
    }
}

fn update_state_from_editor(
    interactions: InteractionIterMut,
    editable_values: &EditableValues,
    node_editor: &AflakNodeEditor,
) {
    for (id, interaction) in interactions {
        if editable_values.contains_key(id) {
            let t_idx = editable_values.get(id).unwrap();
            if let Some(value) = node_editor.constant_node_value(*t_idx) {
                if let Err(e) = match value {
                    IOValue::Integer(i) => interaction.set_value(*i),
                    IOValue::Float(f) => interaction.set_value(*f),
                    IOValue::Float2(f) => interaction.set_value(*f),
                    IOValue::Float3(f) => interaction.set_value(*f),
                    IOValue::Roi(_) => Ok(()),
                    value => Err(format!("Cannot convert value '{:?}'", value)),
                } {
                    eprintln!("Could not update state from editor: {}", e);
                }
            } else {
                eprintln!("No constant node found for transform '{:?}'", t_idx);
            }
        } else {
            eprintln!("'{:?}' not found in store", id);
        }
    }
}

fn update_editor_from_state(
    value_iter: ValueIter,
    store: &mut EditableValues,
    node_editor: &mut AflakNodeEditor,
) {
    for (id, value) in value_iter {
        use aflak_plot::Value;
        let val = match value {
            Value::Integer(i) => IOValue::Integer(i),
            Value::Float(f) => IOValue::Float(f),
            Value::Float2(f) => IOValue::Float2(f),
            Value::Float3(f) => IOValue::Float3(f),
            Value::FinedGrainedROI(pixels) => IOValue::Roi(ROI::PixelList(pixels)),
            Value::Line(pixels) => IOValue::Roi(ROI::PixelList(pixels)),
            Value::Circle(pixels) => IOValue::Roi(ROI::PixelList(pixels)),
        };
        if store.contains_key(id) {
            let t_idx = *store.get(id).unwrap();
            node_editor.update_constant_node(t_idx, val);
        } else {
            let t_idx = node_editor.create_constant_node(val);
            store.insert(*id, t_idx);
        }
    }
}

/// Used to compute the ID of a texture
fn hash_outputid(id: OutputId) -> usize {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut h = DefaultHasher::new();
    id.hash(&mut h);
    h.finish() as usize
}

impl MenuBar for String {
    fn visualize<F>(&self, ctx: OutputWindowCtx<'_, '_, '_, '_, '_, '_, F>)
    where
        F: glium::backend::Facade,
    {
        ctx.ui.text(self);
    }

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ExportError> {
        write_to_file_as_display(path, self)?;
        Ok(())
    }

    fn save_obj<P: AsRef<Path>>(&self, _: P) -> Result<(), ExportError> {
        Ok(())
    }

    const EXTENSION: &'static str = "txt";
}

impl MenuBar for i64 {
    fn visualize<F>(&self, ctx: OutputWindowCtx<'_, '_, '_, '_, '_, '_, F>)
    where
        F: glium::backend::Facade,
    {
        ctx.ui.text(format!("{}", self));
    }

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ExportError> {
        write_to_file_as_display(path, self)?;
        Ok(())
    }

    fn save_obj<P: AsRef<Path>>(&self, _: P) -> Result<(), ExportError> {
        Ok(())
    }

    const EXTENSION: &'static str = "txt";
}

impl MenuBar for f32 {
    fn visualize<F>(&self, ctx: OutputWindowCtx<'_, '_, '_, '_, '_, '_, F>)
    where
        F: glium::backend::Facade,
    {
        ctx.ui.text(format!("{}", self));
    }

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ExportError> {
        write_to_file_as_display(path, self)?;
        Ok(())
    }

    fn save_obj<P: AsRef<Path>>(&self, _: P) -> Result<(), ExportError> {
        Ok(())
    }

    const EXTENSION: &'static str = "txt";
}

impl MenuBar for [f32; 2] {
    fn visualize<F>(&self, ctx: OutputWindowCtx<'_, '_, '_, '_, '_, '_, F>)
    where
        F: glium::backend::Facade,
    {
        ctx.ui.text(format!("{:?}", self));
    }

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ExportError> {
        write_to_file_as_debug(path, self)?;
        Ok(())
    }

    fn save_obj<P: AsRef<Path>>(&self, _: P) -> Result<(), ExportError> {
        Ok(())
    }

    const EXTENSION: &'static str = "txt";
}

impl MenuBar for [f32; 3] {
    fn visualize<F>(&self, ctx: OutputWindowCtx<'_, '_, '_, '_, '_, '_, F>)
    where
        F: glium::backend::Facade,
    {
        ctx.ui.text(format!("{:?}", self));
    }

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ExportError> {
        write_to_file_as_debug(path, self)?;
        Ok(())
    }

    fn save_obj<P: AsRef<Path>>(&self, _: P) -> Result<(), ExportError> {
        Ok(())
    }

    const EXTENSION: &'static str = "txt";
}

impl MenuBar for bool {
    fn visualize<F>(&self, ctx: OutputWindowCtx<'_, '_, '_, '_, '_, '_, F>)
    where
        F: glium::backend::Facade,
    {
        ctx.ui.text(format!("{}", self));
    }

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ExportError> {
        write_to_file_as_display(path, self)?;
        Ok(())
    }

    fn save_obj<P: AsRef<Path>>(&self, _: P) -> Result<(), ExportError> {
        Ok(())
    }

    const EXTENSION: &'static str = "txt";
}

impl MenuBar for Path {
    fn visualize<F>(&self, ctx: OutputWindowCtx<'_, '_, '_, '_, '_, '_, F>)
    where
        F: glium::backend::Facade,
    {
        ctx.ui.text(format!("{:?}", self));
    }

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ExportError> {
        write_to_file_as_display(path, &self.to_string_lossy())?;
        Ok(())
    }

    fn save_obj<P: AsRef<Path>>(&self, _: P) -> Result<(), ExportError> {
        Ok(())
    }

    const EXTENSION: &'static str = "txt";
}

impl MenuBar for primitives::WcsArray {
    fn file_submenu(&self, ui: &Ui, window: &mut OutputWindow, output: OutputId) {
        match &self.scalar().ndim() {
            2 => {
                if MenuItem::new(im_str!("Save to obj (heightmap)")).build(ui) {
                    let path = self.file_name(output);
                    if let Err(e) = self.save_obj(path) {
                        eprintln!("Error on saving output: '{}'", e);
                    //errors.push(Box::new(e));
                    } else {
                        println!("successfully saved");
                    }
                }
            }
            _ => {}
        }
        match self.scalar().ndim() {
            1 | 2 => {
                let has_wcs_data = self.wcs().is_some();
                MenuItem::new(im_str!("Show pixels"))
                    .enabled(has_wcs_data)
                    .build_with_ref(ui, &mut window.show_pixels);
                if !has_wcs_data && ui.is_item_hovered() {
                    ui.tooltip_text("Data has no WCS metadata attached.");
                }
            }
            _ => {}
        }
    }

    fn visualize<F>(&self, ctx: OutputWindowCtx<'_, '_, '_, '_, '_, '_, F>)
    where
        F: glium::backend::Facade,
    {
        use primitives::ndarray::Dimension;
        let ui = &ctx.ui;
        match self.scalar().dim().ndim() {
            0 => {
                let arr = self.array();
                let val = arr.scalar()[[]];
                let unit = arr.unit().repr();
                ui.text(format!("{} {}", val, unit));
            }
            1 => {
                let state = &mut ctx.window.image1d_state;
                update_state_from_editor(
                    state.stored_values_mut(),
                    &ctx.window.editable_values,
                    ctx.node_editor,
                );
                let unit = self.array().unit().repr();
                let transform = if ctx.window.show_pixels {
                    None
                } else {
                    match (self.axes(), self.wcs()) {
                        (Some(axes), Some(wcs)) => {
                            let axis = &axes[0];
                            Some(AxisTransform::new(axis.name(), axis.unit(), move |t| {
                                wcs.pix2world([t, 0.0, 0.0, 0.0])[0]
                            }))
                        }
                        _ => None,
                    }
                };
                if let Err(e) = ui.image1d(&self.scalar1(), "", unit, transform.as_ref(), state) {
                    ui.text(format!("Error on drawing plot! {}", e))
                }
                update_editor_from_state(
                    state.stored_values(),
                    &mut ctx.window.editable_values,
                    ctx.node_editor,
                );
            }
            2 => {
                let state = &mut ctx.window.image2d_state;
                update_state_from_editor(
                    state.stored_values_mut(),
                    &ctx.window.editable_values,
                    &ctx.node_editor,
                );
                let texture_id = TextureId::from(hash_outputid(ctx.output));
                let (x_transform, y_transform) = if ctx.window.show_pixels {
                    (None, None)
                } else {
                    match (self.axes(), self.wcs()) {
                        (Some(axes), Some(wcs)) => {
                            let axis0 = &axes[0];
                            let axis1 = &axes[1];
                            (
                                Some({
                                    AxisTransform::new(axis0.name(), axis0.unit(), move |t| {
                                        wcs.pix2world([t, 0.0, 0.0, 0.0])[0]
                                    })
                                }),
                                Some(AxisTransform::new(axis0.name(), axis1.unit(), {
                                    let max_height =
                                        (self.scalar().dim().as_array_view().first().unwrap() - 1)
                                            as f32;
                                    move |t| wcs.pix2world([0.0, max_height - t, 0.0, 0.0])[1]
                                })),
                            )
                        }
                        _ => (None, None),
                    }
                };
                let unit = self.array().unit().repr();
                let new_incoming_image = match state.image_created_on() {
                    Some(image_created_on) => ctx.created_on > image_created_on,
                    None => true,
                };
                if new_incoming_image {
                    let value_ref: ArcRef<_> = ctx.value.clone().into();
                    let image_ref = value_ref.map(|value| {
                        if let IOValue::Image(image) = value {
                            image.scalar()
                        } else {
                            unreachable!("Expect an Image")
                        }
                    });
                    if let Err(e) = state.set_image(
                        image_ref,
                        ctx.created_on,
                        ctx.gl_ctx,
                        texture_id,
                        ctx.textures,
                    ) {
                        ui.text(format!("Error on creating image! {}", e));
                    }
                }
                if let Err(e) = ui.image2d(
                    ctx.gl_ctx,
                    ctx.textures,
                    texture_id,
                    unit,
                    x_transform.as_ref(),
                    y_transform.as_ref(),
                    state,
                ) {
                    ui.text(format!("Error on drawing image! {}", e));
                }
                update_editor_from_state(
                    state.stored_values(),
                    &mut ctx.window.editable_values,
                    ctx.node_editor,
                );
            }
            _ => {
                ui.text(format!(
                    "Unimplemented for image of dimension {}",
                    self.scalar().ndim()
                ));
            }
        }
    }

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ExportError> {
        let arr = self.scalar();
        // 0-dim ndarrays contain a single scalar value, however they should be
        // treated as a 1-dimensional data array of length 1 when embedded as
        // FITS file.
        let shape = if arr.ndim() == 0 { &[1] } else { arr.shape() };
        Fits::create(
            path,
            Hdu::new(
                shape,
                arr.as_slice()
                    .expect("Could not get slice out of array")
                    .to_owned(),
            ),
        )?;
        Ok(())
    }

    fn save_obj<P: AsRef<Path>>(&self, _path: P) -> Result<(), ExportError> {
        use primitives::ndarray::Dimension;
        let arr = self.scalar();
        if arr.ndim() != 2 {
            /*error!*/
            Ok(())
        } else {
            let mut min = std::f32::MAX;
            let mut max = std::f32::MIN;
            let dim = arr.dim();
            let shape = dim.as_array_view();
            let new_size: Vec<_> = shape.iter().cloned().collect();
            println!("{:?}", new_size);
            for i in arr {
                min = min.min(*i);
                max = max.max(*i);
            }
            let zero_point = (-min / (max - min) * 255.0) as u8;
            println!("{}", zero_point);
            let mut data: Vec<u8> = Vec::with_capacity(new_size[0] * new_size[1] * 3);
            let mut buf = vec![0; new_size[0] * new_size[1]];
            let mut c = 0;
            for i in arr {
                buf[c] = ((i - min) / (max - min) * 255.0) as u8;
                c += 1;
            }
            for d in buf {
                data.push(d);
                data.push(d);
                data.push(d);
                data.push(d);
            }
            let mut i = 0;
            let mut colors = vec![[0; 4]; (new_size[0] * new_size[1]) as usize];
            for color in data.chunks(4) {
                if color.len() == 3 {
                    colors[i] = [color[0] << 0, color[1] << 0, color[2] << 0, 255];
                } else {
                    colors[i] = [color[0] << 0, color[1] << 0, color[2] << 0, color[3] << 0];
                }
                i += 1;
            }
            let mut chunk = Chunk::new(
                [0.0, 0.0, 0.0],
                new_size[0] as usize,
                new_size[1] as usize,
                1,
            );

            for x in 0..new_size[0] as usize {
                for y in 0..new_size[1] as usize {
                    let color = colors[x + y * new_size[0] as usize];
                    let value = (color[0] | color[1]) as f32 / 255.0;
                    chunk.set(x, y, 0, value);
                }
            }
            let mut result = vec![];
            result.push(chunk);
            let mut meshes = vec![];
            for chunk in result.iter() {
                let mesh = HeightMapBaker::bake(chunk);
                if let Some(mesh) = mesh {
                    meshes.push((mesh, chunk.position()));
                }
            }
            export(meshes, &format!("test"), zero_point);
            /*convert to chunk*/
            /*give data to gaiku-3d*/
            Ok(())
        }
    }

    const EXTENSION: &'static str = "fits";
}

fn write_to_file_as_display<P: AsRef<Path>, T: fmt::Display>(path: P, t: &T) -> io::Result<()> {
    let buf = format!("{}\n", t);
    write_to_file_as_bytes(path, buf.as_bytes())
}
fn write_to_file_as_debug<P: AsRef<Path>, T: fmt::Debug>(path: P, t: &T) -> io::Result<()> {
    let buf = format!("{:?}\n", t);
    write_to_file_as_bytes(path, buf.as_bytes())
}
fn write_to_file_as_bytes<P: AsRef<Path>>(path: P, buf: &[u8]) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(buf)
}
fn to_obj(mesh: &Mesh, position: &Point3<f64>, name: &str) -> Object {
    let mut vertices = vec![];
    let mut indices = vec![];

    for vertex in mesh.vertices.iter() {
        let x = vertex.x as f64 + position.x as f64;
        let y = vertex.y as f64 + position.y as f64;
        let z = vertex.z as f64 + position.z as f64;
        vertices.push((x, y, z));
    }

    for i in (0..mesh.indices.len()).step_by(3) {
        indices.push((mesh.indices[i], mesh.indices[i + 1], mesh.indices[i + 2]))
    }

    Object {
        name: name.to_owned(),
        vertices: vertices
            .into_iter()
            .map(|(x, y, z)| Vertex { x, y, z })
            .collect(),
        tex_vertices: vec![],
        normals: vec![],
        geometry: vec![Geometry {
            material_name: Some("hoge".to_string()),
            shapes: indices
                .into_iter()
                .map(|(x, y, z)| Shape {
                    primitive: Primitive::Triangle(
                        (x, None, None),
                        (y, None, None),
                        (z, None, None),
                    ),
                    groups: vec![],
                    smoothing_groups: vec![],
                })
                .collect(),
        }],
    }
}

fn export(data: Vec<(Mesh, &Point3<f64>)>, name: &str, zero_point: u8) {
    let mut objects = vec![];
    let mut max = std::f64::MIN;
    for (index, (mesh, position)) in data.iter().enumerate() {
        for vc in mesh.vertices.iter() {
            let mut it = vc.iter();
            it.next();
            let y = it.next().unwrap();
            max = max.max(*y as f64);
        }
        let obj = to_obj(mesh, position, &format!("mesh_{}", index));
        objects.push(obj);
    }

    let zero_ref = max * zero_point as f64 / 255.0;

    objects.push(Object {
        name: "ReferencePlane".to_owned(),
        vertices: vec![
            (0.0, zero_ref, 0.0),
            (74.0, zero_ref, 0.0),
            (74.0, zero_ref, 74.0),
            (0.0, zero_ref, 74.0),
        ]
        .into_iter()
        .map(|(x, y, z)| Vertex { x, y, z })
        .collect(),
        tex_vertices: vec![],
        normals: vec![],
        geometry: vec![Geometry {
            material_name: None,
            shapes: vec![(0, 1, 2), (0, 2, 3)]
                .into_iter()
                .map(|(x, y, z)| Shape {
                    primitive: Primitive::Triangle(
                        (x, None, None),
                        (y, None, None),
                        (z, None, None),
                    ),
                    groups: vec![],
                    smoothing_groups: vec![],
                })
                .collect(),
        }],
    });

    let set = ObjSet {
        material_library: Some("hoge".to_string()),
        objects,
    };

    obj_exporter::export_to_file(&set, format!("{}.obj", name)).unwrap();
}

#[derive(Debug)]
pub enum ExportError {
    IOError(io::Error),
}

impl fmt::Display for ExportError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ExportError::IOError(e) => write!(fmt, "{}", e),
        }
    }
}

impl error::Error for ExportError {
    /// description is deprecated. See https://github.com/rust-lang/rust/issues/44842
    /// Implement for compilation to succeed on older compilers.
    fn description(&self) -> &str {
        "ExportError"
    }
}

impl From<io::Error> for ExportError {
    fn from(e: io::Error) -> Self {
        ExportError::IOError(e)
    }
}
