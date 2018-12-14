use std::collections::HashMap;
use std::error;
use std::time::Instant;

use glium;
use imgui::{ImString, ImTexture, Ui, Window};
use owning_ref::ArcRef;

use aflak_plot::{
    imshow::{self, Textures, UiImage2d},
    plot::{self, UiImage1d},
    AxisTransform, InteractionId, InteractionIterMut, ValueIter,
};
use cake::{OutputId, TransformIdx, VariantName};
use primitives::{ndarray, IOValue, SuccessOut, ROI};

use aflak::AflakNodeEditor;

#[derive(Default)]
pub struct OutputWindow {
    image1d_state: plot::State,
    image2d_state: imshow::State<ArcRef<IOValue, ndarray::ArrayD<f32>>>,
    editable_values: EditableValues,
    show_pixels: bool,
}

type EditableValues = HashMap<InteractionId, TransformIdx>;

impl OutputWindow {
    pub fn draw<'ui, F>(
        &mut self,
        ui: &'ui Ui,
        output: OutputId,
        window: Window<'ui, '_>,
        node_editor: &mut AflakNodeEditor,
        gl_ctx: &F,
        textures: &mut Textures,
    ) -> Vec<Box<error::Error>>
    where
        F: glium::backend::Facade,
    {
        let compute_state = node_editor.compute_output(output);
        match compute_state {
            None => {
                Initializing.draw(ui, window);
                vec![]
            }
            Some(Err(e)) => {
                e.draw(ui, window);
                vec![]
            }
            Some(Ok(result)) => {
                let created_on = SuccessOut::created_on(&result);
                let value = SuccessOut::take(result);
                let ctx = OutputWindowCtx {
                    ui,
                    output,
                    value: &value,
                    window: self,
                    created_on,
                    node_editor,
                    gl_ctx,
                    textures,
                };
                match &*value {
                    IOValue::Str(ref string) => string.draw(ctx, window),
                    IOValue::Integer(integer) => integer.draw(ctx, window),
                    IOValue::Float(float) => float.draw(ctx, window),
                    IOValue::Float2(floats) => floats.draw(ctx, window),
                    IOValue::Float3(floats) => floats.draw(ctx, window),
                    IOValue::Bool(b) => b.draw(ctx, window),
                    IOValue::Image(ref image) => image.draw(ctx, window),
                    IOValue::Fits(ref fits) => {
                        fits.draw(ui, window);
                        vec![]
                    }
                    _ => {
                        let unimplemented = Unimplemented {
                            variant: value.variant_name(),
                        };
                        unimplemented.draw(ui, window);
                        vec![]
                    }
                }
            }
        }
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

struct OutputWindowCtx<'ui, 'val, 'w, 'tex, 'ed, 'gl, F: 'gl> {
    ui: &'ui Ui<'ui>,
    output: OutputId,
    value: &'val ::std::sync::Arc<IOValue>,
    window: &'w mut OutputWindow,
    created_on: Instant,
    node_editor: &'ed mut AflakNodeEditor,
    gl_ctx: &'gl F,
    textures: &'tex mut Textures,
}

struct Initializing;

impl Visualizable for Initializing {
    fn visualize(&self, ui: &Ui) {
        ui.text("Initialiazing...");
    }
}

struct Unimplemented {
    variant: &'static str,
}

impl Visualizable for Unimplemented {
    fn visualize(&self, ui: &Ui) {
        ui.text(format!(
            "Cannot visualize variable of type '{}'!",
            self.variant
        ));
    }
}

use std::fmt;

impl<E: fmt::Display> Visualizable for cake::DSTError<E> {
    fn visualize(&self, ui: &Ui) {
        ui.text_wrapped(&ImString::new(format!("{}", self)));
    }
}

trait Visualizable {
    fn visualize(&self, ui: &Ui);

    fn draw<'ui>(&self, ui: &'ui Ui, window: Window<'ui, '_>) {
        window.build(|| self.visualize(ui));
    }
}

trait MenuBar {
    fn visualize<F>(&self, ctx: OutputWindowCtx<'_, '_, '_, '_, '_, '_, F>)
    where
        F: glium::backend::Facade;

    fn draw<'ui, F>(
        &self,
        ctx: OutputWindowCtx<'ui, '_, '_, '_, '_, '_, F>,
        window: Window<'ui, '_>,
    ) -> Vec<Box<error::Error>>
    where
        F: glium::backend::Facade,
    {
        let mut errors = vec![];
        window.menu_bar(true).build(|| {
            errors = MenuBar::menu_bar(self, ctx.ui, ctx.output, ctx.window);
            MenuBar::visualize(self, ctx);
        });
        errors
    }

    fn file_submenu(&self, _: &Ui, _: &mut OutputWindow) {}

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ExportError>;

    const EXTENSION: &'static str;

    fn file_name(&self, output: OutputId) -> String {
        format!("output-{}.{}", output.id(), Self::EXTENSION)
    }

    fn menu_bar(
        &self,
        ui: &Ui,
        output: OutputId,
        window: &mut OutputWindow,
    ) -> Vec<Box<error::Error>> {
        let mut errors: Vec<Box<error::Error>> = vec![];

        let mut output_saved_success_popup = false;
        ui.menu_bar(|| {
            ui.menu(im_str!("File")).build(|| {
                if ui.menu_item(im_str!("Save")).build() {
                    let path = self.file_name(output);
                    if let Err(e) = self.save(path) {
                        eprintln!("Error on saving output: '{}'", e);
                        errors.push(Box::new(e));
                    } else {
                        output_saved_success_popup = true;
                    }
                }
                self.file_submenu(ui, window);
            });
        });

        if output_saved_success_popup {
            ui.open_popup(im_str!("FITS export completed!"));
        }
        ui.popup_modal(im_str!("FITS export completed!")).build(|| {
            ui.text(format!(
                "File saved with success to '{}'.",
                self.file_name(output)
            ));
            if ui.button(im_str!("Close"), (0.0, 0.0)) {
                ui.close_current_popup();
            }
        });

        errors
    }
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

impl MenuBar for primitives::WcsArray {
    fn file_submenu(&self, ui: &Ui, window: &mut OutputWindow) {
        ui.menu_item(im_str!("Show pixels"))
            .selected(&mut window.show_pixels)
            .build();
    }

    fn visualize<F>(&self, ctx: OutputWindowCtx<'_, '_, '_, '_, '_, '_, F>)
    where
        F: glium::backend::Facade,
    {
        use primitives::ndarray::Dimension;
        let ui = &ctx.ui;
        match self.scalar().dim().ndim() {
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
                    match (self.cunits(), self.wcs()) {
                        (Some(units), Some(wcs)) => {
                            Some(AxisTransform::new(units[0].repr(), move |t| {
                                wcs.pix2world([t, 0.0, 0.0, 0.0])[0]
                            }))
                        }
                        _ => None,
                    }
                };
                if let Err(e) = ui.image1d(&self.scalar1(), &unit, transform, state) {
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
                let texture_id = ImTexture::from(hash_outputid(ctx.output));
                let (x_transform, y_transform) = if ctx.window.show_pixels {
                    (None, None)
                } else {
                    match (self.cunits(), self.wcs()) {
                        (Some(units), Some(wcs)) => (
                            Some(AxisTransform::new(units[0].repr(), move |t| {
                                wcs.pix2world([t, 0.0, 0.0, 0.0])[0]
                            })),
                            Some(AxisTransform::new(units[1].repr(), {
                                let max_height =
                                    (self.scalar().dim().as_array_view().first().unwrap() - 1)
                                        as f32;
                                move |t| wcs.pix2world([0.0, max_height - t, 0.0, 0.0])[1]
                            })),
                        ),
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
                    x_transform,
                    y_transform,
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
        Fits::create(
            path,
            Hdu::new(
                arr.shape(),
                arr.as_slice()
                    .expect("Could not get slice out of array")
                    .to_owned(),
            ),
        )?;
        Ok(())
    }

    const EXTENSION: &'static str = "fits";
}

impl Visualizable for Fits {
    fn visualize(&self, ui: &Ui) {
        let mut has_hdus = false;
        for (i, hdu) in self.iter().enumerate() {
            use primitives::fitrs::HeaderValue::*;
            use std::borrow::Cow;

            has_hdus = true;

            let tree_name = match hdu.value("EXTNAME") {
                Some(CharacterString(extname)) => ImString::new(extname.as_str()),
                _ => {
                    if i == 0 {
                        im_str!("Primary HDU").to_owned()
                    } else {
                        ImString::new(format!("Hdu #{}", i))
                    }
                }
            };

            ui.push_id(i as i32);
            ui.tree_node(&tree_name).build(|| {
                for (key, value) in hdu {
                    ui.text(key);
                    if let Some(value) = value {
                        ui.same_line(150.0);
                        let value = match value {
                            CharacterString(s) => Cow::Borrowed(s.as_str()),
                            Logical(true) => Cow::Borrowed("True"),
                            Logical(false) => Cow::Borrowed("False"),
                            IntegerNumber(i) => Cow::Owned(format!("{}", i)),
                            RealFloatingNumber(f) => Cow::Owned(format!("{:E}", f)),
                            ComplexIntegerNumber(a, b) => Cow::Owned(format!("{} + {}i", a, b)),
                            ComplexFloatingNumber(a, b) => {
                                Cow::Owned(format!("{:E} + {:E}i", a, b))
                            }
                        };
                        ui.text(value);
                    }
                    ui.separator();
                }
            });
            ui.pop_id();
        }
        if !has_hdus {
            ui.text("Input Fits appears invalid. No HDU could be found.");
        }
    }
}

use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use primitives::fitrs::{Fits, Hdu};

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

impl Error for ExportError {
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
