# Project structure

Hyperspeedcube consists of many crates, split into several categories and ordered from highest-level to lowest-level.

Each crate contains a `README.md` with more info about it. In particular, I recommend reading [`hyperpuzzle/README.md`](hyperpuzzle/README.md)

Most of the crates fall into three categories:

- Crates with the prefix `hsc` are primarily for Hyperspeedcube, but may be used by other projects to interoperate with Hyperspeedcube or borrow parts of its coed. These include preferences (`hsc_prefs`), log files (`hsc_log`), graphics (`hsc_draw`), etc.
- Crates with the prefix `hypuz` or `hyperpuzzle` and are intended to be useful to build alternative tooling related to twisty puzzles. These include Hyperpuzzlescript (`hyperpuzzlescript`), notation (`hypuz_notation`), puzzle backends (`hyperpuzzle_core`, `hyperpuzzle_impl_*`, `hyperpuzzle`), etc.
- Crates with the prefix `hyper` are intended to be useful even for non-puzzle-related projects. These include general mathematics (`hypermath`), group theory (`hypergroup`), shape-cutting (`hypershape`), etc.

## Crates

These are organized roughly from highest-level to lowest-level.

### User interface

- `hyperspeedcube`, the main entry point for the [egui] application and CLI
- `hsc_view`, which tracks puzzle state, interaction, and animations
- `hsc_draw`, which renders the puzzle to a texture using [wgpu]
- `hsc_cli_types`, which provides types used by the CLI via JSON

[wgpu]: https://wgpu.rs/
[egui]: https://github.com/emilk/egui

### Filesystem

- `hsc_log`, which handles log file serialization & deserialization and verification
- `hsc_paths`, which finds locations on disk where files should be read from or saved to
- `hsc_prefs`, which manages user preferences
- `hsc_stats`, which manages local storage of solves and personal-bests

### Puzzle simulation

- `hyperpuzzle`, which aggregates puzzles defined using all backends (currently `hypuz_impl_euclid`)
- `hypuz_impl_euclid`, a puzzle engine for Hyperpuzzlescript
- `hyperpuzzlescript`, which is a puzzle backend using a domain-specific programming language for puzzle definitions that is extensible by adding new engines
- `hypuz_core`, which defines types for puzzles that can be implemented by puzzle backends
- `hypuz_notation`, which provides puzzle move notation parsing

### Math & geometry

- `hypershape`, which provides shape slicing and other geometric algorithms
- `hypergroup`, which provides [Group Theory](https://en.wikipedia.org/wiki/Group_theory) primitives
- `hypermath`, which provides vectors, matrices, projective geometric algebra primitives, and common multi-purpose data structures

### Build dependencies

- `hsc_strings`, which generates `hyperspeedcube/src/locales.rs` containing localizations for Hyperspeedcube from `hyperspeedcube/locales/*.kdl`.

### Leaderboards

- `hsc_leaderboards`, which manages authentication and submission to the [Hypercubing Leaderboards](https://lb.hypercubing.xyz/)
- `timecheck`, which provides functions to contact a Randomness Beacon and a Time Stamp Authority for verified scramble timestamps and completion timestamps respectively

### Other

- `hyperkdl`/`hyperkdl_derive`, which provide derive macros for serializing/deserializing structs to KDL
- `hypuz_util`, which provides general utility functions
