//! # aflak - Advanced Framework for Learning Astrophysical Knowledge
//!
extern crate clap;
extern crate glium;
#[macro_use]
extern crate imgui;
extern crate imgui_glium_renderer;
extern crate owning_ref;

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
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process;

use imgui::ImString;

use node_editor::NodeEditor;

use aflak::Aflak;
use constant_editor::MyConstantEditor;

const CLEAR_COLOR: [f32; 4] = [0.05, 0.05, 0.05, 1.0];

fn main() -> support::Result<()> {
    env::set_var("WINIT_HIDPI_FACTOR", "1");

    let transformations_ref = Box::leak(Box::new(
        primitives::TRANSFORMATIONS.iter().collect::<Vec<_>>(),
    ));
    let transformations = transformations_ref.as_slice();

    let matches = cli::build_cli().version(version()).get_matches();

    let import_data = match open_buffer(&matches) {
        Ok(buf) => buf,
        Err(e) => {
            if let Some(file_name) = matches.value_of("load") {
                eprintln!("Error on opening file '{}': {}", file_name, e);
            } else {
                eprintln!("Error on opening buffer: {}", e);
            }
            process::exit(1)
        }
    };

    let node_editor =
        match NodeEditor::from_export_buf(import_data, transformations, MyConstantEditor) {
            Ok(editor) => editor,
            Err(e) => {
                eprintln!("Import failed! Initialize empty node editor.\n{}", e);
                NodeEditor::new(transformations, MyConstantEditor)
            }
        };

    let mut aflak = Aflak::init(node_editor);

    let config = support::AppConfig {
        title: format!("aflak {}", env!("CARGO_PKG_VERSION")),
        clear_color: CLEAR_COLOR,
        ini_filename: Some(ImString::new("aflak.ini")),
        maximized: true,
        ..Default::default()
    };
    support::run(config, |ui, gl_ctx, textures| {
        aflak.node_editor(ui);
        aflak.output_windows(ui, gl_ctx, textures);
        aflak.show_errors(ui);
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

fn open_buffer(matches: &clap::ArgMatches) -> Result<Box<Read>, io::Error> {
    let fits = matches.value_of("fits");
    let fits_path = path_clean_up(fits, "file.fits");
    if let Some(template_name) = matches.value_of("template") {
        let template = match template_name {
            "waveform" => templates::show_frame_and_wave(fits_path),
            "equivalent_width" => templates::show_equivalent_width(fits_path),
            "fits_cleaning" => templates::show_fits_cleaning(fits_path),
            _ => unreachable!("Got '{}', an unexpected result.", template_name),
        };
        Ok(Box::new(template))
    } else if let Some(file_path) = matches.value_of("load") {
        if file_path == "-" {
            Ok(Box::new(StdinReader::default()))
        } else if file_path.ends_with(".fits") {
            let fits = primitives::fitrs::Fits::open(file_path)?;
            if let Some(primary_hdu) = fits.get(0) {
                if let Some(primitives::fitrs::HeaderValue::CharacterString(string)) =
                    primary_hdu.value("AFLAPROV")
                {
                    let read = ::std::io::Cursor::new(string.to_owned());
                    Ok(Box::new(read))
                } else {
                    panic!("Could not find AFLAPROV key in FITS file '{}'. Was this FITS file exported by aflak?", file_path)
                }
            } else {
                panic!("Selected FITS file '{}' does not appear to have a primary HDU. It is probably not a FITS file.", file_path)
            }
        } else {
            let file = fs::File::open(file_path)?;
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
