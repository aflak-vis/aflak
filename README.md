# Aflak - アフラーク

**Aflak** - A visualization environment to analyze astronomical datasets
by providing a visual programming language interface.

[![Build Status](https://travis-ci.org/aflak-vis/aflak.svg?branch=master)](https://travis-ci.org/aflak-vis/aflak)
[![Latest release on crates.io](https://meritbadge.herokuapp.com/aflak)](https://crates.io/crates/aflak)

![Screenshot of Aflak](images/aflak-screen.png)

**IN ACTIVE DEVELOPMENT: Features and API highly unstable!**

## Getting started

Minimum Rust version: 1.26.2.

Install the rust toolchain with [rustup](https://rustup.rs/).
Then clone the git repository.

You need to initialize the git submodules.

```sh
cd aflak
git submodule update --init --recursive
```

## Build

```sh
cd aflak/src
cargo build --release
```

## Run aflak

```sh
cd aflak/src
cargo run --release -- -f <FITS_FILE>
```

**NB**: The first time you run aflak, the window layout may not be what you
prefer. You may want to resize / move some windows with the mouse the
very first time your run aflak.
Hopefully, aflak remembers the arrangement of your windows between sessions.

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
