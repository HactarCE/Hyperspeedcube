# Project structure

Hyperspeedcube consists of many crates, split into several categories and ordered from highest-level to lowest-level. Each crate contains a `README.md` with more info about it.

[dev.hypercubing.xyz](https://dev.hypercubing.xyz/) may contain other useful information.

## User interface

- `hyperspeedcube`, which provides a UI frontend using [egui]
- `hyperpuzzle_view`, which tracks puzzle state, interaction, and animations
- `hyperdraw`, which renders the puzzle to a texture using [wgpu]

[wgpu]: https://wgpu.rs/
[egui]: https://github.com/emilk/egui

## Filesystem

- `hyperprefs`, which manages user preferences
- `hyperpuzzle_log`, which handles log file serialization & deserialization
- `hyperpaths`, which finds locations on disk where files should be read from or saved to

## Puzzle simulation

- `hyperpuzzle`, which aggregates puzzles defined using all backends (currently just `hyperpuzzle_lua`)
- `hyperpuzzle_lua`, a puzzle backend which defines puzzles using a Lua API
- `hyperpuzzle_core`, which defines types for puzzles

## Math & geometry

- `hypershape`, which provides shape slicing and other geometric algorithms
- `hypermath`, which provides vectors, matrices, projective geometric algebra primitives, and common multi-purpose data structures

## Build dependencies

- `hyperstrings`, which generates localizations for Hyperspeedcube (currently only American English) from `hyperspeedcube/locales/*.kdl` into the `hyperspeedcube/src/locales.rs`.
