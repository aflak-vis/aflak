# Aflak - アフラーク

**Aflak** - A visualization environment to analyze astronomical datasets
by providing a visual programming language interface.

![Screenshot of Aflak](images/aflak-screen.png)

**IN ACTIVE DEVELOPMENT: Features and API highly unstable!**

## Getting started

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
cargo run --release
```

**NB**: The first time you run aflak, the window layout will be completely
messed up. You will need to resize all the windows with the mouse only the
very first time your run aflak. This needs to be fixed.
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
cargo test
```

## TODO

- **imgui_file_explorer** crate
- Aflak complete front-end
- Write output to FITS files
- Output window automatic layout: Current default layout is completely dumb.
- Zooming in node editor
