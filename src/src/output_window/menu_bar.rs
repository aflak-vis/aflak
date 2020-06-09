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
    IOValue, ROI,
};

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

    fn file_submenu(&self, _: &Ui, _: &mut OutputWindow) {}

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
                self.file_submenu(ui, window);
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

    const EXTENSION: &'static str = "txt";
}

impl MenuBar for ROI {
    fn visualize<F>(&self, ctx: OutputWindowCtx<'_, '_, '_, '_, '_, '_, F>)
    where
        F: glium::backend::Facade,
    {
        ctx.ui.text_wrapped(&im_str!("{:?}", self));
    }

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ExportError> {
        write_to_file_as_display(path, &format!("{:?}", self))?;
        Ok(())
    }

    const EXTENSION: &'static str = "txt";
}

impl MenuBar for primitives::WcsArray {
    fn file_submenu(&self, ui: &Ui, window: &mut OutputWindow) {
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
