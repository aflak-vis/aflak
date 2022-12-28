use std::error;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::Instant;

use glium;

use imgui::{MenuItem, TextureId, Ui, Window};
use owning_ref::ArcRef;

use crate::aflak_plot::{
    imshow::{Textures, UiImage2d},
    persistence_diagram::UiPersistenceDiagram,
    plot::UiImage1d,
    plot_colormap::UiColorMap,
    scatter_lineplot::UiScatter,
    three::UiImage3d,
    AxisTransform, InteractionId, InteractionIterMut, ValueIter,
};
use crate::cake::{OutputId, TransformIdx};
use crate::primitives::{
    self,
    fitrs::{Fits, Hdu},
    IOValue, PATHS, ROI,
};

use implot::Context;

use super::{AflakNodeEditor, EditableValues, OutputWindow};

/// Catch-all object for variables used by output window during render
pub struct OutputWindowCtx<'ui, 'val, 'w, 'tex, 'ed, 'gl, 'p, F: 'gl> {
    pub ui: &'ui Ui<'ui>,
    pub output: OutputId,
    pub value: &'val ::std::sync::Arc<IOValue>,
    pub window: &'w mut OutputWindow,
    pub created_on: Instant,
    pub node_editor: &'ed mut AflakNodeEditor,
    pub gl_ctx: &'gl F,
    pub textures: &'tex mut Textures,
    pub plotcontext: &'p Context,
    pub copying: &'w mut Option<(InteractionId, TransformIdx)>,
    pub attaching: &'w mut Option<(OutputId, TransformIdx, usize)>,
}

/// Similar to Visualizable, excepts that the types that implements this trait
/// are more complex.
/// The can be exported to disk and display options can be included in a menu
/// bar.
pub trait MenuBar {
    fn visualize<F>(&self, ctx: OutputWindowCtx<'_, '_, '_, '_, '_, '_, '_, F>)
    where
        F: glium::backend::Facade;

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ExportError>;

    const EXTENSION: &'static str;

    fn draw<'ui, F, T>(
        &self,
        ctx: OutputWindowCtx<'ui, '_, '_, '_, '_, '_, '_, F>,
        window: Window<'_, T>,
    ) -> Vec<Box<dyn error::Error>>
    where
        F: glium::backend::Facade,
        T: AsRef<str>,
    {
        let mut errors = vec![];
        window
            .menu_bar(true)
            .scroll_bar(false)
            .scrollable(false)
            .build(ctx.ui, || {
                errors = MenuBar::menu_bar(self, ctx.ui, ctx.output, ctx.window);
                MenuBar::visualize(self, ctx);
            });
        errors
    }

    fn file_submenu(&self, _: &Ui, _: &mut OutputWindow) {}
    fn other_menu(&self, _: &Ui, _: &mut OutputWindow) {}
    fn zoom_menu(&self, _: &Ui, _: &mut OutputWindow) {}
    fn histogram_menu(&self, _: &Ui, _: &mut OutputWindow) {}

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
            if let Some(menu) = ui.begin_menu_with_enabled(format!("File"), true) {
                if MenuItem::new(format!("Save")).build(ui) {
                    let path = self.file_name(output);
                    if let Err(e) = self.save(path) {
                        eprintln!("Error on saving output: '{}'", e);
                        errors.push(Box::new(e));
                    } else {
                        output_saved_success_popup = true;
                    }
                }
                self.file_submenu(ui, window);
                menu.end();
            }
            self.other_menu(ui, window);
        });

        if output_saved_success_popup {
            ui.open_popup(format!("Export completed!"));
        }
        ui.popup_modal(format!("Export completed!")).build(ui, || {
            ui.text(format!(
                "File saved with success to '{}'.",
                self.file_name(output)
            ));
            if ui.button(format!("Close")) {
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
    use aflak_plot::Interaction;

    for (id, interaction) in interactions {
        if editable_values.contains_key(id) {
            let t_idx = editable_values.get(id).unwrap();
            if let Some(macro_id) = t_idx.macro_id() {
                if let Some(macr) = node_editor.macros.get_macro(macro_id) {
                    let macr = macr.read();
                    if let Some(value) = macr.get_constant_value(*t_idx) {
                        if let Err(e) = match value {
                            IOValue::Integer(i) => interaction.set_value(*i),
                            IOValue::Float(f) => interaction.set_value(*f),
                            IOValue::Float2(f) => interaction.set_value(*f),
                            IOValue::Float3(f) => interaction.set_value(*f),
                            IOValue::Float3x3(f) => interaction.set_value(*f),
                            IOValue::Roi(r) => match r {
                                primitives::ROI::All => Ok(()),
                                primitives::ROI::PixelList(p) => match interaction {
                                    Interaction::FinedGrainedROI(r) => {
                                        let changed = r.changed;
                                        interaction.set_value(((*p).clone(), changed))
                                    }
                                    _ => Ok(()),
                                },
                            },
                            value => Err(format!("Cannot convert value '{:?}'", value)),
                        } {
                            eprintln!("Could not update state from editor: {}", e);
                        }
                    } else {
                        eprintln!("No constant node found for transform '{:?}'", t_idx);
                    }
                }
            } else {
                if let Some(value) = node_editor.constant_node_value(*t_idx) {
                    if let Err(e) = match value {
                        IOValue::Integer(i) => interaction.set_value(*i),
                        IOValue::Float(f) => interaction.set_value(*f),
                        IOValue::Float2(f) => interaction.set_value(*f),
                        IOValue::Float3(f) => interaction.set_value(*f),
                        IOValue::Float3x3(f) => interaction.set_value(*f),
                        IOValue::Roi(_) => Ok(()),
                        IOValue::ColorLut(l) => interaction.set_value(l.clone()),
                        value => Err(format!("Cannot convert value '{:?}'", value)),
                    } {
                        eprintln!("Could not update state from editor: {}", e);
                    }
                } else {
                    eprintln!("No constant node found for transform '{:?}'", t_idx);
                }
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
    for (id, interaction, value) in value_iter {
        use aflak_plot::{Interaction, Value};
        let change_flag = match interaction {
            Interaction::HorizontalLine(h) => h.moving,
            Interaction::VerticalLine(v) => v.moving,
            Interaction::FinedGrainedROI(r) => r.changed,
            _ => true,
        };
        let val = match value {
            Value::Integer(i) => IOValue::Integer(i),
            Value::Float(f) => IOValue::Float(f),
            Value::Float2(f) => IOValue::Float2(f),
            Value::Float3(f) => IOValue::Float3(f),
            Value::Float3x3(f) => IOValue::Float3x3(f),
            Value::FinedGrainedROI(pixels) => IOValue::Roi(ROI::PixelList(pixels.0)),
            Value::Line(pixels) => IOValue::Roi(ROI::PixelList(pixels)),
            Value::Circle(pixels) => IOValue::Roi(ROI::PixelList(pixels)),
            Value::ColorLut(lut) => IOValue::ColorLut(lut),
        };
        if store.contains_key(id) {
            if change_flag {
                let t_idx = *store.get(id).unwrap();
                node_editor.update_constant_node(t_idx, val);
            }
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
    fn visualize<F>(&self, mut ctx: OutputWindowCtx<'_, '_, '_, '_, '_, '_, '_, F>)
    where
        F: glium::backend::Facade,
    {
        if let Some(attaching) = ctx.attaching {
            if attaching.0 == ctx.output {
                attach_failued(&mut ctx, &"String");
            }
        }
        ctx.ui.text(self);
    }

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ExportError> {
        write_to_file_as_display(path, self)?;
        Ok(())
    }

    const EXTENSION: &'static str = "txt";
}

impl MenuBar for i64 {
    fn visualize<F>(&self, mut ctx: OutputWindowCtx<'_, '_, '_, '_, '_, '_, '_, F>)
    where
        F: glium::backend::Facade,
    {
        if let Some(attaching) = ctx.attaching {
            if attaching.0 == ctx.output {
                attach_failued(&mut ctx, &"Integer");
            }
        }
        ctx.ui.text(format!("{}", self));
    }

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ExportError> {
        write_to_file_as_display(path, self)?;
        Ok(())
    }

    const EXTENSION: &'static str = "txt";
}

impl MenuBar for f32 {
    fn visualize<F>(&self, mut ctx: OutputWindowCtx<'_, '_, '_, '_, '_, '_, '_, F>)
    where
        F: glium::backend::Facade,
    {
        if let Some(attaching) = ctx.attaching {
            if attaching.0 == ctx.output {
                attach_failued(&mut ctx, &"Float");
            }
        }
        ctx.ui.text(format!("{}", self));
    }

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ExportError> {
        write_to_file_as_display(path, self)?;
        Ok(())
    }

    const EXTENSION: &'static str = "txt";
}

impl MenuBar for [f32; 2] {
    fn visualize<F>(&self, mut ctx: OutputWindowCtx<'_, '_, '_, '_, '_, '_, '_, F>)
    where
        F: glium::backend::Facade,
    {
        if let Some(attaching) = ctx.attaching {
            if attaching.0 == ctx.output {
                attach_failued(&mut ctx, &"Float2");
            }
        }
        ctx.ui.text(format!("{:?}", self));
    }

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ExportError> {
        write_to_file_as_debug(path, self)?;
        Ok(())
    }

    const EXTENSION: &'static str = "txt";
}

impl MenuBar for [f32; 3] {
    fn visualize<F>(&self, mut ctx: OutputWindowCtx<'_, '_, '_, '_, '_, '_, '_, F>)
    where
        F: glium::backend::Facade,
    {
        if let Some(attaching) = ctx.attaching {
            if attaching.0 == ctx.output {
                attach_failued(&mut ctx, &"Float3");
            }
        }
        ctx.ui.text(format!("{:?}", self));
    }

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ExportError> {
        write_to_file_as_debug(path, self)?;
        Ok(())
    }

    const EXTENSION: &'static str = "txt";
}

impl MenuBar for [[f32; 3]; 3] {
    fn visualize<F>(&self, mut ctx: OutputWindowCtx<'_, '_, '_, '_, '_, '_, '_, F>)
    where
        F: glium::backend::Facade,
    {
        if let Some(attaching) = ctx.attaching {
            if attaching.0 == ctx.output {
                attach_failued(&mut ctx, &"Float3x3");
            }
        }
        ctx.ui.text(format!("{:?}", self));
    }

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ExportError> {
        write_to_file_as_debug(path, self)?;
        Ok(())
    }

    const EXTENSION: &'static str = "txt";
}

impl MenuBar for bool {
    fn visualize<F>(&self, mut ctx: OutputWindowCtx<'_, '_, '_, '_, '_, '_, '_, F>)
    where
        F: glium::backend::Facade,
    {
        if let Some(attaching) = ctx.attaching {
            if attaching.0 == ctx.output {
                attach_failued(&mut ctx, &"Bool");
            }
        }
        ctx.ui.text(format!("{}", self));
    }

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ExportError> {
        write_to_file_as_display(path, self)?;
        Ok(())
    }

    const EXTENSION: &'static str = "txt";
}

impl MenuBar for PATHS {
    fn visualize<F>(&self, mut ctx: OutputWindowCtx<'_, '_, '_, '_, '_, '_, '_, F>)
    where
        F: glium::backend::Facade,
    {
        if let Some(attaching) = ctx.attaching {
            if attaching.0 == ctx.output {
                attach_failued(&mut ctx, &"Paths");
            }
        }
        ctx.ui.text_wrapped(&format!("{:?}", self));
    }

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ExportError> {
        write_to_file_as_display(path, &format!("{:?}", self))?;
        Ok(())
    }

    const EXTENSION: &'static str = "txt";
}

impl MenuBar for ROI {
    fn visualize<F>(&self, mut ctx: OutputWindowCtx<'_, '_, '_, '_, '_, '_, '_, F>)
    where
        F: glium::backend::Facade,
    {
        if let Some(attaching) = ctx.attaching {
            if attaching.0 == ctx.output {
                attach_failued(&mut ctx, &"Roi");
            }
        }
        ctx.ui.text_wrapped(&format!("{:?}", self));
    }

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ExportError> {
        write_to_file_as_display(path, &format!("{:?}", self))?;
        Ok(())
    }

    const EXTENSION: &'static str = "txt";
}

impl MenuBar for &(usize, Vec<(f32, [u8; 3])>) {
    fn other_menu(&self, ui: &Ui, window: &mut OutputWindow) {
        if let Some(menu) = ui.begin_menu_with_enabled(format!("Color Mode"), true) {
            if MenuItem::new(format!("RGB"))
                .build_with_ref(ui, &mut window.colormap_state.colormode[0])
            {
                if window.colormap_state.colormode[0] == true {
                    window.colormap_state.colormode[1] = false;
                } else {
                    window.colormap_state.colormode[0] = true;
                }
            };
            if MenuItem::new(format!("HSV"))
                .build_with_ref(ui, &mut window.colormap_state.colormode[1])
            {
                if window.colormap_state.colormode[1] == true {
                    window.colormap_state.colormode[0] = false;
                } else {
                    window.colormap_state.colormode[1] = true;
                }
            };
            menu.end();
        }
    }
    fn visualize<F>(&self, mut ctx: OutputWindowCtx<'_, '_, '_, '_, '_, '_, '_, F>)
    where
        F: glium::backend::Facade,
    {
        ctx.ui.text_wrapped(&format!("{:?}", self));
        if let Some(attaching) = ctx.attaching {
            if attaching.0 == ctx.output {
                if attaching.2 != 3 {
                    attach_failued(&mut ctx, &"ColorLut");
                }
            }
        }
        let ui = &ctx.ui;
        let state = &mut ctx.window.colormap_state;
        update_state_from_editor(
            state.stored_values_mut(),
            &ctx.window.editable_values,
            ctx.node_editor,
        );
        if let Err(e) = ui.colormap(
            self,
            "",
            state,
            &mut ctx.copying,
            &mut ctx.window.editable_values,
            &mut ctx.attaching,
            ctx.output,
            &ctx.node_editor,
        ) {
            ui.text(format!("Error on drawing colormap! {}", e))
        }
        update_editor_from_state(
            state.stored_values(),
            &mut ctx.window.editable_values,
            ctx.node_editor,
        );
    }

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ExportError> {
        write_to_file_as_display(path, &format!("{:?}", self))?;
        Ok(())
    }

    const EXTENSION: &'static str = "txt";
}

impl MenuBar for primitives::PersistencePairs {
    fn visualize<F>(&self, mut ctx: OutputWindowCtx<'_, '_, '_, '_, '_, '_, '_, F>)
    where
        F: glium::backend::Facade,
    {
        if let Some(attaching) = ctx.attaching {
            if attaching.0 == ctx.output {
                attach_failued(&mut ctx, &"PersistencePairs");
            }
        }
        let ui = &ctx.ui;
        let state = &mut ctx.window.persistence_diagram_state;
        let plot_ui = ctx.plotcontext.get_plot_ui();
        update_state_from_editor(
            state.stored_values_mut(),
            &ctx.window.editable_values,
            &ctx.node_editor,
        );
        if let Err(e) = ui.persistence_diagram(
            &self,
            &plot_ui,
            state,
            &mut ctx.copying,
            &mut ctx.window.editable_values,
            &mut ctx.attaching,
            ctx.created_on,
            ctx.output,
        ) {
            ui.text(format!("Error on drawing plot! {}", e))
        }
        update_editor_from_state(
            state.stored_values(),
            &mut ctx.window.editable_values,
            ctx.node_editor,
        );
    }

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ExportError> {
        write_to_file_as_debug(path, self)?;
        Ok(())
    }

    const EXTENSION: &'static str = "txt";
}

impl MenuBar for primitives::WcsArray {
    fn file_submenu(&self, ui: &Ui, window: &mut OutputWindow) {
        match &self.tag() {
            None => match self.scalar().ndim() {
                1 | 2 => {
                    let has_wcs_data = self.wcs().is_some();
                    MenuItem::new(format!("Show pixels"))
                        .enabled(has_wcs_data)
                        .build_with_ref(ui, &mut window.show_pixels);
                    if !has_wcs_data && ui.is_item_hovered() {
                        ui.tooltip_text("Data has no WCS metadata attached.");
                    }
                }
                _ => {}
            },
            Some(tag) => match tag.as_ref() {
                "BPT" => match self.scalar().ndim() {
                    2 => {}
                    _ => {}
                },
                _ => {}
            },
        }
    }

    fn other_menu(&self, ui: &Ui, window: &mut OutputWindow) {
        match &self.tag() {
            None => match self.scalar().ndim() {
                1 => {
                    if let Some(menu) = ui.begin_menu_with_enabled(format!("Others"), true) {
                        MenuItem::new(format!("Axes Option"))
                            .build_with_ref(ui, &mut window.image1d_state.show_axis_option);
                        menu.end();
                    }
                }
                2 => {
                    if let Some(menu) = ui.begin_menu_with_enabled(format!("Window"), true) {
                        self.zoom_menu(ui, window);
                        menu.end();
                    }
                    if let Some(menu) = ui.begin_menu_with_enabled(format!("Histogram"), true) {
                        self.histogram_menu(ui, window);
                        menu.end();
                    }
                    if let Some(menu) = ui.begin_menu_with_enabled(format!("Contour"), true) {
                        MenuItem::new(format!("Contour Control"))
                            .build_with_ref(ui, &mut window.image2d_state.show_contour);
                        menu.end();
                    }
                    if let Some(_) = &self.topology() {
                        if let Some(menu) = ui.begin_menu_with_enabled(format!("Topology"), true) {
                            MenuItem::new(format!("Critical Points")).build_with_ref(
                                ui,
                                &mut window.image2d_state.show_tp_critical_points,
                            );
                            MenuItem::new(format!("Separatrices Points")).build_with_ref(
                                ui,
                                &mut window.image2d_state.show_tp_separatrices_points,
                            );
                            MenuItem::new(format!("Connection"))
                                .build_with_ref(ui, &mut window.image2d_state.show_tp_connections);
                            menu.end();
                        }
                    }
                    if let Some(menu) = ui.begin_menu_with_enabled(format!("Others"), true) {
                        MenuItem::new(format!("Approx Line"))
                            .build_with_ref(ui, &mut window.image2d_state.show_approx_line);
                        MenuItem::new(format!("Axes Option"))
                            .build_with_ref(ui, &mut window.image2d_state.show_axis_option);
                        menu.end();
                    }
                }
                3 => {
                    if let Some(menu) =
                        ui.begin_menu_with_enabled(format!("Transfer Function"), true)
                    {
                        let have_topology = &self.topology().is_some();
                        if let Some(menu) = ui.begin_menu_with_enabled(format!("ColorMap"), true) {
                            MenuItem::new(format!("Edit"))
                                .build_with_ref(ui, &mut window.image3d_state.show_colormapedit);
                            menu.end();
                        }
                        if let Some(menu) = ui.begin_menu_with_enabled(format!("Isosurface"), true)
                        {
                            MenuItem::new(format!("Single isosurface"))
                                .build_with_ref(ui, &mut window.image3d_state.show_single_contour);
                            if ui.is_item_clicked() {
                                if !window.image3d_state.show_single_contour {
                                    window.image3d_state.critical_isosurface = false;
                                    window.image3d_state.representative_isosurface = false;
                                }
                                window.image3d_state.single_contour_clicked = true;
                            }
                            if let Some(menu) =
                                ui.begin_menu_with_enabled(format!("Use topology"), *have_topology)
                            {
                                MenuItem::new(format!("Critical isosurface")).build_with_ref(
                                    ui,
                                    &mut window.image3d_state.critical_isosurface,
                                );
                                if ui.is_item_clicked() && !window.image3d_state.critical_isosurface
                                {
                                    window.image3d_state.show_single_contour = false;
                                    window.image3d_state.representative_isosurface = false;
                                }
                                MenuItem::new(format!("Representative isosurface")).build_with_ref(
                                    ui,
                                    &mut window.image3d_state.representative_isosurface,
                                );
                                if ui.is_item_clicked()
                                    && !window.image3d_state.representative_isosurface
                                {
                                    window.image3d_state.show_single_contour = false;
                                    window.image3d_state.critical_isosurface = false;
                                }
                                menu.end();
                            }
                            menu.end();
                        }
                        MenuItem::new(format!("Brightness Settings"))
                            .build_with_ref(ui, &mut window.image3d_state.show_tf_parameters);
                        menu.end();
                    }
                }
                _ => {}
            },
            Some(tag) => match tag.as_ref() {
                "BPT" => match self.scalar().ndim() {
                    2 => {
                        if let Some(menu) = ui.begin_menu_with_enabled(format!("Graph"), true) {
                            MenuItem::new(format!("Graph Editor")).build_with_ref(
                                ui,
                                &mut window.scatter_lineplot_state.show_graph_editor,
                            );
                            menu.end();
                        }
                        if let Some(menu) = ui.begin_menu_with_enabled(format!("Option"), true) {
                            if MenuItem::new(format!("Show all data points")).build_with_ref(
                                ui,
                                &mut window.scatter_lineplot_state.show_all_point,
                            ) {
                                window.scatter_lineplot_state.editor_changed = true;
                            }
                            menu.end();
                        }
                    }
                    _ => {}
                },
                "color_image" => match self.scalar().ndim() {
                    3 => {
                        if let Some(menu) = ui.begin_menu_with_enabled(format!("Window"), true) {
                            self.zoom_menu(ui, window);
                            menu.end();
                        }
                        if let Some(menu) = ui.begin_menu_with_enabled(format!("Histogram"), true) {
                            self.histogram_menu(ui, window);
                            menu.end();
                        }
                    }
                    _ => {}
                },
                _ => {}
            },
        }
    }

    fn zoom_menu(&self, ui: &Ui, window: &mut OutputWindow) {
        if let Some(menu) = ui.begin_menu_with_enabled(format!("Zoom"), true) {
            if MenuItem::new(format!("Fit"))
                .build_with_ref(ui, &mut window.image2d_state.zoomkind[0])
            {
                if window.image2d_state.zoomkind[0] == true {
                    window.image2d_state.zoomkind[1] = false;
                    window.image2d_state.zoomkind[2] = false;
                    window.image2d_state.zoomkind[3] = false;
                } else {
                    window.image2d_state.zoomkind[0] = true;
                }
            }
            if MenuItem::new(format!("50%"))
                .build_with_ref(ui, &mut window.image2d_state.zoomkind[1])
            {
                if window.image2d_state.zoomkind[1] == true {
                    window.image2d_state.zoomkind[0] = false;
                    window.image2d_state.zoomkind[2] = false;
                    window.image2d_state.zoomkind[3] = false;
                } else {
                    window.image2d_state.zoomkind[1] = true;
                }
            }
            if MenuItem::new(format!("100%"))
                .build_with_ref(ui, &mut window.image2d_state.zoomkind[2])
            {
                if window.image2d_state.zoomkind[2] == true {
                    window.image2d_state.zoomkind[0] = false;
                    window.image2d_state.zoomkind[1] = false;
                    window.image2d_state.zoomkind[3] = false;
                } else {
                    window.image2d_state.zoomkind[2] = true;
                }
            }
            if MenuItem::new(format!("200%"))
                .build_with_ref(ui, &mut window.image2d_state.zoomkind[3])
            {
                if window.image2d_state.zoomkind[3] == true {
                    window.image2d_state.zoomkind[0] = false;
                    window.image2d_state.zoomkind[1] = false;
                    window.image2d_state.zoomkind[2] = false;
                } else {
                    window.image2d_state.zoomkind[3] = true;
                }
            }
            menu.end();
        }
    }

    fn histogram_menu(&self, ui: &Ui, window: &mut OutputWindow) {
        MenuItem::new(format!("Logscale"))
            .build_with_ref(ui, &mut window.image2d_state.hist_logscale);
    }

    fn visualize<F>(&self, mut ctx: OutputWindowCtx<'_, '_, '_, '_, '_, '_, '_, F>)
    where
        F: glium::backend::Facade,
    {
        use crate::primitives::ndarray::Dimension;
        match &self.tag() {
            None => match self.scalar().dim().ndim() {
                0 => {
                    if let Some(attaching) = ctx.attaching {
                        if attaching.0 == ctx.output {
                            attach_failued(&mut ctx, &"String");
                        }
                    }
                    let arr = self.array();
                    let val = arr.scalar()[[]];
                    let unit = arr.unit().repr();
                    let ui = &ctx.ui;
                    ui.text(format!("{} {}", val, unit));
                }
                1 => {
                    let attaching = &ctx.attaching;
                    let output = &ctx.output;
                    if let Some(attaching) = attaching {
                        if attaching.0 == *output {
                            if attaching.2 != 1 {
                                attach_failued(&mut ctx, &"1D plot");
                            }
                        }
                    }
                    let ui = &ctx.ui;
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
                    if let Err(e) = ui.image1d(
                        &self.scalar1(),
                        "",
                        unit,
                        transform.as_ref(),
                        state,
                        &mut ctx.copying,
                        &mut ctx.window.editable_values,
                        &mut ctx.attaching,
                        ctx.output,
                        &ctx.node_editor,
                    ) {
                        ui.text(format!("Error on drawing plot! {}", e))
                    }
                    update_editor_from_state(
                        state.stored_values(),
                        &mut ctx.window.editable_values,
                        ctx.node_editor,
                    );
                }
                2 => {
                    let attaching = &ctx.attaching;
                    let output = &ctx.output;
                    if let Some(attaching) = attaching {
                        if attaching.0 == *output {
                            if attaching.2 != 0 && attaching.2 != 1 {
                                attach_failued(&mut ctx, &"2D Image");
                            }
                        }
                    }
                    let ui = &ctx.ui;
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
                                    Some(AxisTransform::new(axis1.name(), axis1.unit(), {
                                        move |t| wcs.pix2world([0.0, t, 0.0, 0.0])[1]
                                    })),
                                )
                            }
                            _ => (None, None),
                        }
                    };
                    let unit = self.array().unit().repr();
                    state.topology = self.topology().clone();
                    let new_incoming_image = match state.image_created_on() {
                        Some(image_created_on) => ctx.created_on > image_created_on,
                        None => true,
                    };
                    if new_incoming_image {
                        state.zoom_init();
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
                        &mut ctx.copying,
                        &mut ctx.window.editable_values,
                        &mut ctx.attaching,
                        ctx.output,
                        &ctx.node_editor,
                    ) {
                        ui.text(format!("Error on drawing image! {}", e));
                    }
                    update_editor_from_state(
                        state.stored_values(),
                        &mut ctx.window.editable_values,
                        ctx.node_editor,
                    );
                }
                3 => {
                    let ui = &ctx.ui;
                    let state = &mut ctx.window.image3d_state;
                    let arr = self.array();
                    let val = arr.scalar().view();
                    let texture_id = TextureId::from(hash_outputid(ctx.output));
                    let new_incoming_image = match state.image_created_on() {
                        Some(image_created_on) => ctx.created_on > image_created_on,
                        None => true,
                    };
                    state.topology = self.topology().clone();
                    if new_incoming_image {
                        state.new(ctx.created_on);
                    }
                    ui.image3d(&val, texture_id, ctx.textures, ctx.gl_ctx, state);
                }
                _ => {
                    let ui = &ctx.ui;
                    ui.text(format!(
                        "Unimplemented for image of dimension {}",
                        self.scalar().ndim()
                    ));
                }
            },
            Some(tag) => match tag.as_ref() {
                "BPT" => match self.scalar().dim().ndim() {
                    2 => {
                        let ui = &ctx.ui;
                        let state = &mut ctx.window.scatter_lineplot_state;
                        let plot_ui = ctx.plotcontext.get_plot_ui();
                        update_state_from_editor(
                            state.stored_values_mut(),
                            &ctx.window.editable_values,
                            &ctx.node_editor,
                        );
                        if let Err(e) = ui.scatter(
                            &self.scalar2(),
                            &plot_ui,
                            Some(&AxisTransform::new("X Axis", "m", |x| x)),
                            Some(&AxisTransform::new("Y Axis", "m", |y| y)),
                            state,
                            &mut ctx.copying,
                            &mut ctx.window.editable_values,
                            &mut ctx.attaching,
                            ctx.output,
                        ) {
                            ui.text(format!("Error on drawing plot! {}", e))
                        }
                        update_editor_from_state(
                            state.stored_values(),
                            &mut ctx.window.editable_values,
                            ctx.node_editor,
                        );
                    }
                    _ => {
                        let ui = &ctx.ui;
                        ui.text(format!(
                            "Unimplemented for scatter of dimension {}",
                            self.scalar().ndim()
                        ));
                    }
                },
                "scatter" => match self.scalar().dim().ndim() {
                    2 => {
                        let ui = &ctx.ui;
                        let state = &mut ctx.window.scatter_lineplot_state;
                        let plot_ui = ctx.plotcontext.get_plot_ui();
                        if let Err(e) = ui.scatter(
                            &self.scalar2(),
                            &plot_ui,
                            Some(&AxisTransform::new("X Axis", "m", |x| x)),
                            Some(&AxisTransform::new("Y Axis", "m", |y| y)),
                            state,
                            &mut ctx.copying,
                            &mut ctx.window.editable_values,
                            &mut ctx.attaching,
                            ctx.output,
                        ) {
                            ui.text(format!("Error on drawing plot! {}", e))
                        }
                    }
                    _ => {
                        let ui = &ctx.ui;
                        ui.text(format!(
                            "Unimplemented for scatter of dimension {}",
                            self.scalar().ndim()
                        ));
                    }
                },
                "color_image" => match self.scalar().dim().ndim() {
                    3 => {
                        let ui = &ctx.ui;
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
                                            AxisTransform::new(
                                                axis0.name(),
                                                axis0.unit(),
                                                move |t| wcs.pix2world([t, 0.0, 0.0, 0.0])[0],
                                            )
                                        }),
                                        Some(AxisTransform::new(axis1.name(), axis1.unit(), {
                                            move |t| wcs.pix2world([0.0, t, 0.0, 0.0])[1]
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
                            state.zoom_init();
                            let value_ref: ArcRef<_> = ctx.value.clone().into();
                            let image_ref = value_ref.map(|value| {
                                if let IOValue::Image(image) = value {
                                    image.scalar()
                                } else {
                                    unreachable!("Expect an Image")
                                }
                            });
                            if let Err(e) = state.set_color_image(
                                image_ref,
                                ctx.created_on,
                                ctx.gl_ctx,
                                texture_id,
                                ctx.textures,
                            ) {
                                ui.text(format!("Error on creating image! {}", e));
                            }
                        }
                        if let Err(e) = ui.color_image(
                            ctx.gl_ctx,
                            ctx.textures,
                            texture_id,
                            unit,
                            x_transform.as_ref(),
                            y_transform.as_ref(),
                            state,
                            &mut ctx.copying,
                            &mut ctx.window.editable_values,
                            &mut ctx.attaching,
                            ctx.output,
                            &ctx.node_editor,
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
                        let ui = &ctx.ui;
                        ui.text(format!(
                            "Unimplemented for color image of dimension {}",
                            self.scalar().ndim()
                        ));
                    }
                },
                _ => {
                    let ui = &ctx.ui;
                    ui.text(format!(
                        "Unimplemented for visualization method {}",
                        self.scalar().ndim()
                    ));
                }
            },
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

fn attach_failued<F>(ctx: &mut OutputWindowCtx<'_, '_, '_, '_, '_, '_, '_, F>, viztype: &str)
where
    F: glium::backend::Facade,
{
    ctx.ui.open_popup(format!("Attach failued"));
    ctx.ui
        .popup_modal(format!("Attach failued"))
        .build(ctx.ui, || {
            let kind = match ctx.attaching.unwrap().2 {
                0 => "horizontal line",
                1 => "vertical line",
                2 => "Region Of Interest",
                3 => "Color Map",
                _ => "Unimplemented Interaction",
            };
            ctx.ui.text(format!(
                "Attach failued, {} cannot be used in {}",
                kind, viztype
            ));
            if ctx.ui.button(format!("Close")) {
                *ctx.attaching = None;
                ctx.ui.close_current_popup();
            }
        });
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
