# Aflak - アフラーク

**Aflak** - A visualization environment to analyze astronomical datasets
by providing a visual programming language interface.

[![Build Status](https://travis-ci.org/aflak-vis/aflak.svg?branch=master)](https://travis-ci.org/aflak-vis/aflak)
[![Latest release on crates.io](https://meritbadge.herokuapp.com/aflak)](https://crates.io/crates/aflak)

![Screenshot of Aflak](images/aflak-screen.png)

**IN ACTIVE DEVELOPMENT: Features and API highly unstable!**

## Getting started

Minimum Rust version: 1.28.0.

Install the rust toolchain with [rustup](https://rustup.rs/).

## Quick install (nightly)

```sh
cargo install --git https://github.com/aflak-vis/aflak aflak
# Open a FITS file with aflak
aflak -f <FITS_FILE>
# See CLI help
aflak --help
```

You may find a demo video [here](https://vimeo.com/290328343).

**Disclaimer**: Most testing until now has been done with FITS files sampled
from the [SDSS MaNGA](https://www.sdss.org/surveys/manga/) dataset.
Other FITS files may or may not work. Feedback and bug report is welcome.

**NB**:
- The first time you run aflak, the window layout may not be what you
prefer. You may want to resize / move some windows with the mouse the
very first time your run aflak.
Hopefully, aflak remembers the arrangement of your windows between sessions.
- It is advised to use aflak with a large screen more 2000-pixel large
for a better experience. 3000-pixel is even better!

## Update

If aflak is already installed, just append the `--force` flag to the `cargo`
command in order to overwrite the current install of aflak with a new one.

```sh
cargo install --force --git https://github.com/aflak-vis/aflak aflak
```

## Slower install

Clone the git repository.
You will need to initialize the git submodules.

```sh
git clone https://github.com/aflak-vis/aflak
cd aflak/src
git submodule update --init --recursive
cargo install --path .
```

## Build

```sh
cd aflak/src
cargo build --release
```

## Run aflak from source

```sh
cd aflak/src
cargo run --release -- -f <FITS_FILE>
```

## Development

This repo has several crates each with a specific and defined objective.
Each of the crates is documented. Please refer to the doc.

- **aflak_cake** (*Computational mAKE*): Manage node graph (back-end).

```sh
cd aflak/src/aflak_cake
# Open the doc
cargo doc --open
```
- **aflak_primitives**: Define transformation types and primitives for use in
astrophysics.
- **node_editor**: Node editor built on *aflak_cake* and *imgui*.

## Tests

```sh
cd aflak/src
cargo test --all
```

## TODO

- Zooming in node editor
