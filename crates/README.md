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
- `hyperpuzzle_log`, which handles log file serialization & deserialization and verification
- `hyperpaths`, which finds locations on disk where files should be read from or saved to

## Puzzle simulation

- `hyperpuzzle`, which aggregates puzzles defined using all backends (currently just `hyperpuzzle_impl_nd_euclid`)
- `hyperpuzzle_impl_nd_euclid`, a puzzle engine for Hyperpuzzlescript
- `hyperpuzzlescript`, which is a puzzle backend using a domain-specific programming language for puzzle definitions that is extensible by adding new engines
- `hyperpuzzle_core`, which defines types for puzzles that can be implemented by puzzle backends

## Math & geometry

- `hypershape`, which provides shape slicing and other geometric algorithms
- `hypermath`, which provides vectors, matrices, projective geometric algebra primitives, and common multi-purpose data structures

## Build dependencies

- `hyperstrings`, which generates localizations for Hyperspeedcube (currently only American English) from `hyperspeedcube/locales/*.kdl` into the `hyperspeedcube/src/locales.rs`.

## Leaderboards

- `hypercubing_leaderboards_auth`, which manages authentication to the [Hypercubing Leaderboards](https://lb.hypercubing.xyz/)
- `timecheck`, which provides functions to contact a Randomness Beacon and a Time Stamp Authority for verified scramble timestamps and completion timestamps respectively
