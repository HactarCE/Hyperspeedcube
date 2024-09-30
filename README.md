<img src="https://raw.githubusercontent.com/HactarCE/Hyperspeedcube/main/resources/icon/hyperspeedcube.svg?sanitize=true" alt="Hyperspeedcube logo" width="150" align="right">

# [Hyperspeedcube] [![Release badge]][Release link]

[Dependencies badge]: https://deps.rs/repo/github/HactarCE/Hyperspeedcube/status.svg "Dependencies status"
[Release badge]: https://img.shields.io/github/v/release/HactarCE/Hyperspeedcube
[Release link]: https://github.com/HactarCE/Hyperspeedcube/releases/latest

Hyperspeedcube is a modern, beginner-friendly 3D and 4D Rubik's cube simulator with thousands of puzzles, customizable mouse and keyboard controls, and a [Lua API for creating new puzzles](https://dev.hypercubing.xyz/hsc/puzzle-dev/). It has been used to break numerous speedsolving records and is the software of choice in the [Hypercubing community](https://hypercubing. xyz/).

For more info, see [ajfarkas.dev/hyperspeedcube](https://ajfarkas.dev/hyperspeedcube/)

[Hyperspeedcube]: https://ajfarkas.dev/hyperspeedcube/

## Project structure

This project consists of four crates, each depending on the previous ones:

- `hypermath`, which provides vectors, matrices, projective geometric algebra primitives, and common multi-purpose data structures
- `hypershape`, which provides shape slicing and other geometric algorithms
- `hyperpuzzle`, which provides puzzle construction and simulation via a Lua API
- `hyperspeedcube`, which provides the UI frontend

### Possible future plans

- Move Lua API into `hyperpuzzle_lua` and make `hyperpuzzle` extensible by other crates
- Split `hyperspeedcube` into `hyperspeedcube_core`, `hyperspeedcube_wgpu`, and `hyperspeedcube_egui`.
- Alternatively: by default, `hyperspeedcube` has the `gui` feature enabled. By disabling it, you can use `hyperspeedcube` as a dependency in other projects and build your own frontend.

## License & contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).
