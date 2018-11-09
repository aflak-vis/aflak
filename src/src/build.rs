extern crate clap;

use std::env;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use clap::Shell;

mod cli;

fn main() {
    let outdir = match env::var_os("OUT_DIR") {
        None => return,
        Some(outdir) => outdir,
    };
    let mut app = cli::build_cli();
    app.gen_completions(env!("CARGO_PKG_NAME"), Shell::Bash, &outdir);

    fs::File::create(PathBuf::from(outdir).join("commit-info.txt"))
        .expect("Could not create commit-info.txt file")
        .write_all(commit_info().as_bytes())
        .expect("Could not write to commit-info.txt");
}

// Try to get hash and date of the last commit on a best effort basis. If anything goes wrong
// (git not installed or if this is not a git repository) just return an empty string.
// Thanks rustup for the reference implementation:
// https://github.com/rust-lang-nursery/rustup.rs/blob/dd51ab0/build.rs
fn commit_info() -> String {
    match (commit_hash(), commit_date()) {
        (Ok(hash), Ok(date)) => format!(" ({} {})", hash.trim_right(), date),
        _ => String::new(),
    }
}

fn commit_hash() -> Result<String, Ignore> {
    Ok(String::from_utf8(
        Command::new("git")
            .args(&["rev-parse", "--short=9", "HEAD"])
            .output()?
            .stdout,
    )?)
}

fn commit_date() -> Result<String, Ignore> {
    Ok(String::from_utf8(
        Command::new("git")
            .args(&["log", "-1", "--date=short", "--pretty=format:%cd"])
            .output()?
            .stdout,
    )?)
}

struct Ignore;

impl<E> From<E> for Ignore
where
    E: Error,
{
    fn from(_: E) -> Ignore {
        Ignore
    }
}
