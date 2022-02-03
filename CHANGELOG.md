# Changelog

All notable changes to Hyperspeedcube will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), except for minor stylistic changes to organize features and accomodate named versions. This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html) with respect to the Rust API for `hyperspeedcube`, but the minor and patch numbers may also be incremented for changes that only affect the GUI application.

## [Unreleased]

### Added

- Automatic reloading of preferences file when modified externally

### Changed

- **Preferences file format is not backwards-compatible. Existing custom keybindings will be erased when loading for the first time.**
- Preferences loading is now more lenient with handling invalid/missing values

### Fixed

- HiDPI / display scaling ([#2](https://github.com/HactarCE/Hyperspeedcube/issues/2))
- Configuring font size no longer requires restart

## [0.1.0] - 2022-01-05

### Added

- **Puzzles**
  - Rubik's 3D (3x3x3)
  - Rubik's 4D (3x3x3x3)
- **Customization**
  - Graphics settings
  - View/projection parameters
  - Colors
  - Keybinds for selecting (highlighting) various facets of the puzzles
  - Keybinds for twisting the puzzle
- **Import/export**
  - Can save/load MC4D log files for Rubik's 4D puzzle
- **Undo history**
  - Undo (<kbd>Ctrl</kbd>+<kbd>Z</kbd>)
  - Redo (<kbd>Ctrl</kbd>+<kbd>Y</kbd> or <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Z</kbd>)
