//! # aflak - Advanced Framework for Learning Astrophysical Knowledge
//!
extern crate clap;
extern crate glium;
#[macro_use]
extern crate imgui;
extern crate imgui_glium_renderer;

extern crate aflak_cake as cake;
extern crate aflak_imgui_glium_support as support;
extern crate aflak_plot;
extern crate aflak_primitives as primitives;
extern crate imgui_file_explorer;
extern crate node_editor;

mod aflak;
mod constant_editor;
mod layout;
mod save_output;
mod templates;

use std::env;
use std::path::PathBuf;

use clap::{App, Arg};
use imgui::ImString;

use node_editor::NodeEditor;

use aflak::Aflak;
use constant_editor::MyConstantEditor;

const CLEAR_COLOR: [f32; 4] = [0.05, 0.05, 0.05, 1.0];

fn main() -> support::Result<()> {
    env::set_var("WINIT_HIDPI_FACTOR", "1");

    let transformations_ref = primitives::TRANSFORMATIONS.iter().collect::<Vec<_>>();
    let transformations = transformations_ref.as_slice();

    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("fits")
                .short("f")
                .long("fits")
                .value_name("FITS")
                .help("Set a FITS file to load"),
        ).get_matches();

    let fits = matches.value_of("fits").unwrap_or("file.fits");
    let fits = PathBuf::from(fits);
    let fits = if fits.is_absolute() {
        fits
    } else {
        let pwd = env::current_dir().unwrap_or_default();
        pwd.join(fits)
    };
    let fits = fits.canonicalize().unwrap_or(fits);
    let import_data = templates::show_frame_and_wave(fits);

    let node_editor = NodeEditor::from_export_buf(import_data, transformations, MyConstantEditor)
        .expect("Import failed");

    let mut aflak = Aflak::init(node_editor);

    let config = support::AppConfig {
        title: "aflak".to_owned(),
        clear_color: CLEAR_COLOR,
        ini_filename: Some(ImString::new("aflak.ini")),
        ..Default::default()
    };
    support::run(config, |ui, gl_ctx, textures| {
        aflak.node_editor(ui);
        aflak.output_windows(ui, gl_ctx, textures);
        true
    })
}
