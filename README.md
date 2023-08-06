<img src="https://raw.githubusercontent.com/HactarCE/Hyperspeedcube/main/resources/icon/hyperspeedcube.svg?sanitize=true" alt="Hyperspeedcube logo" width="150" align="right">

# [Hyperspeedcube] [![Release badge]][Release link]

[Dependencies badge]: https://deps.rs/repo/github/HactarCE/Hyperspeedcube/status.svg "Dependencies status"
[Release badge]: https://img.shields.io/github/v/release/HactarCE/Hyperspeedcube
[Release link]: https://github.com/HactarCE/Hyperspeedcube/releases/latest

Hyperspeedcube is a modern, beginner-friendly 3D and 4D Rubik's cube simulator with customizable mouse and keyboard controls and advanced features for speedsolving. It's been used to break numerous speedsolving records.

For more info, see [ajfarkas.dev/hyperspeedcube](https://ajfarkas.dev/hyperspeedcube/)

[Hyperspeedcube]: https://ajfarkas.dev/hyperspeedcube/

## Project structure

This project consists of four crates, each depending on the previous ones:

- `hypermath`, which implements vectors, matrices, conformal geometric algebra primitives, and common multi-purpose data structures
- `hypershape`, which implements shape slicing and other geometric algorithms
- `hyperpuzzle`, which implements puzzle construction and simulation via a Lua API
- `hyperspeedcube`, which implements the UI frontend

### Possible future plans

- Split `hyperspeedcube` into `hyperspeedcube_core`, `hyperspeedcube_wgpu`, and `hyperspeedcube_egui`.
- Alternatively: by default, `hyperspeedcube` has the `gui` feature enabled. By disabling it, you can use `hyperspeedcube` as a dependency in other projects and build your own frontend.
