# Aflak - アフラーク

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
cargo build
```

## Run NodeEditor example

```sh
cd aflak/src/node_editor
cargo run --example empty
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
**node_editor** contains an example called *empty* showing how the front-end
for aflak should be implemented. This is not a full front-end yet!

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
