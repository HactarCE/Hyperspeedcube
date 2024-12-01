<img src="https://raw.githubusercontent.com/HactarCE/Hyperspeedcube/main/resources/icon/hyperspeedcube.svg?sanitize=true" alt="Hyperspeedcube logo" width="150" align="right">

# [Hyperspeedcube] [![Release badge]][Release link]

[Dependencies badge]: https://deps.rs/repo/github/HactarCE/Hyperspeedcube/status.svg "Dependencies status"
[Release badge]: https://img.shields.io/github/v/release/HactarCE/Hyperspeedcube
[Release link]: https://github.com/HactarCE/Hyperspeedcube/releases/latest

Hyperspeedcube is a modern, beginner-friendly 3D and 4D Rubik's cube simulator with thousands of puzzles, customizable mouse and keyboard controls, and a [Lua API for creating new puzzles](https://dev.hypercubing.xyz/hsc/puzzle-dev/). It has been used to break numerous speedsolving records and is the software of choice in the [Hypercubing community](https://hypercubing.xyz/).

For more info, see [ajfarkas.dev/hyperspeedcube](https://ajfarkas.dev/hyperspeedcube/)

[Hyperspeedcube]: https://ajfarkas.dev/hyperspeedcube/

## Project structure

This project consists of many crates, each depending on the previous ones:

- `hypermath`, which provides vectors, matrices, projective geometric algebra primitives, and common multi-purpose data structures
- `hypershape`, which provides shape slicing and other geometric algorithms
- `hyperpuzzle`, which provides puzzle construction and simulation via a Lua API
- `hyperprefs`, which provides user preferences
- `hyperdraw`, which renders the puzzle to a texture using [wgpu]
- `hyperpuzzleview`, which tracks puzzle state, interaction, and animations
- `hyperspeedcube`, which provides a UI frontend based on [egui]

There is also `hyperstrings`, which generates localizations for Hyperspeedcube (currently only American English) from `hyperspeedcube/locales/*.kdl` into the `hyperspeedcube/src/locales.rs`.

[wgpu]: https://wgpu.rs/
[egui]: https://github.com/emilk/egui

## License & contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).
