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
mod cli;
mod constant_editor;
mod layout;
mod save_output;
mod templates;

use std::env;
use std::path::PathBuf;

use imgui::ImString;

use node_editor::NodeEditorApp;

use aflak::Aflak;
use constant_editor::MyConstantEditor;

const CLEAR_COLOR: [f32; 4] = [0.05, 0.05, 0.05, 1.0];

fn main() -> support::Result<()> {
    env::set_var("WINIT_HIDPI_FACTOR", "1");

    let transformations_ref = primitives::TRANSFORMATIONS.iter().collect::<Vec<_>>();
    let transformations = transformations_ref.as_slice();

    let matches = cli::build_cli().version(version()).get_matches();

    let fits = matches.value_of("fits");
    let fits_path = path_clean_up(fits, "file.fits");

    let import_data = match matches.value_of("template") {
        Some("waveform") | None => templates::show_frame_and_wave(fits_path),
        Some("equivalent_width") => templates::show_equivalent_width(fits_path),
        Some(template) => unreachable!("Got '{}', an unexpected result.", template),
    };

    let node_editor =
        match NodeEditorApp::from_export_buf(import_data, transformations, MyConstantEditor) {
            Ok(editor) => editor,
            Err(e) => {
                eprintln!("Import failed! Initialize empty node editor.\n{}", e);
                NodeEditorApp::new(transformations, MyConstantEditor)
            }
        };

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

/// Clean up path from user input.
/// Make local path absolute and attempt to canonize it.
fn path_clean_up(path: Option<&str>, default: &str) -> PathBuf {
    let path = path.unwrap_or(default);
    let path = PathBuf::from(path);
    let path = if path.is_absolute() {
        path
    } else {
        let pwd = env::current_dir().unwrap_or_default();
        pwd.join(path)
    };
    path.canonicalize().unwrap_or(path)
}

fn version() -> &'static str {
    concat!(
        env!("CARGO_PKG_VERSION"),
        include_str!(concat!(env!("OUT_DIR"), "/commit-info.txt"))
    )
}
