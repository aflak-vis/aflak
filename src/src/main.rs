//! # aflak - Advanced Framework for Learning Astrophysical Knowledge
//!
extern crate clap;
extern crate glium;
extern crate imgui;
extern crate imgui_glium_renderer;
extern crate implot;
extern crate owning_ref;

extern crate aflak_cake as cake;
extern crate aflak_imgui_glium_support as support;
extern crate aflak_plot;
extern crate aflak_primitives as primitives;
extern crate imgui_file_explorer;
extern crate imgui_tone_curve;
extern crate node_editor;

mod aflak;
mod cli;
mod constant_editor;
mod file_dialog;
mod layout;
mod output_window;
mod templates;

use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process;

use node_editor::NodeEditor;

use crate::aflak::Aflak;

use implot::Context;

const CLEAR_COLOR: [f32; 4] = [0.05, 0.05, 0.05, 1.0];

fn main() {
    let matches = cli::build_cli().version(version()).get_matches();

    let import_data = match open_buffer(&matches) {
        Ok(buf) => buf,
        Err(e) => {
            if let Some(file_name) = matches.value_of("ron") {
                eprintln!("Error on opening file '{}': {}", file_name, e);
            } else {
                eprintln!("Error on opening buffer: {}", e);
            }
            process::exit(1)
        }
    };

    let node_editor = match NodeEditor::from_export_buf(import_data) {
        Ok(editor) => editor,
        Err(e) => {
            eprintln!("Import failed! Initialize empty node editor.\n{}", e);
            NodeEditor::default()
        }
    };

    let mut aflak = Aflak::init(node_editor);

    let config = support::AppConfig {
        title: format!("aflak {}", env!("CARGO_PKG_VERSION")),
        clear_color: CLEAR_COLOR,
        ini_filename: Some(PathBuf::from("aflak.ini")),
        maximized: true,
        ..Default::default()
    };
    let transformations_ref: Vec<_> = primitives::TRANSFORMATIONS.iter().collect();
    let plotcontext = Context::create();
    support::init(config).main_loop(move |ui, gl_ctx, textures| {
        let transformations = transformations_ref.as_slice();
        aflak.main_menu_bar(ui);
        aflak.node_editor(ui, transformations);
        aflak.output_windows(ui, gl_ctx, textures, &plotcontext);
        aflak.show_errors(ui);
        aflak.file_dialog(ui);
        if aflak.show_metrics {
            ui.show_metrics_window(&mut aflak.show_metrics);
        }
        if aflak.show_bind_manager {
            aflak.bind_manager(ui);
        }
        !aflak.quit
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

fn open_buffer(matches: &clap::ArgMatches) -> Result<Box<dyn Read>, io::Error> {
    let fits = matches.value_of("fits");
    let fits_path = path_clean_up(fits, "file.fits");
    if let Some(template_name) = matches.value_of("template") {
        if let Some(template) = templates::select_template(template_name, fits_path) {
            Ok(Box::new(template))
        } else {
            unreachable!("Got '{}', an unexpected result.", template_name)
        }
    } else if let Some(ron_file) = matches.value_of("ron") {
        if ron_file == "-" {
            Ok(Box::new(StdinReader::default()))
        } else {
            let file = fs::File::open(ron_file)?;
            Ok(Box::new(file))
        }
    } else {
        // Fall back to default template
        let default_template = templates::show_frame_and_wave(fits_path);
        Ok(Box::new(default_template))
    }
}

struct StdinReader {
    r: io::Stdin,
}

impl Default for StdinReader {
    fn default() -> Self {
        Self { r: io::stdin() }
    }
}

impl io::Read for StdinReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut lock = self.r.lock();
        lock.read(buf)
    }
}

fn version() -> &'static str {
    concat!(
        env!("CARGO_PKG_VERSION"),
        include_str!(concat!(env!("OUT_DIR"), "/commit-info.txt"))
    )
}
