extern crate clap;

use std::env;

use clap::Shell;

mod cli;

fn main() {
    let outdir = match env::var_os("OUT_DIR") {
        None => return,
        Some(outdir) => outdir,
    };
    let mut app = cli::build_cli();
    app.gen_completions(env!("CARGO_PKG_NAME"), Shell::Bash, outdir);
}
