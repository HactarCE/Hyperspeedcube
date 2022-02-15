# Changelog

All notable changes to Hyperspeedcube will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), except for minor stylistic changes to organize features and accomodate named versions. This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html) with respect to the Rust API for `hyperspeedcube`, but the minor and patch numbers may also be incremented for changes that only affect the GUI application.

## Unreleased

### Changed

- **Preferences are now saved in `hyperspeedcube.yaml` in the same directory as `hyperspeedcube.exe` by default.** To continue using the old location, create a file named `nonportable` (no file extension) in the same directory as `hyperspeedcube.exe`. The old preferences file can be found in the following locations:
  - Linux: `/home/<user>/.config/hyperspeedcube/hyperspeedcube.yaml`
  - macOS: `/Users/<user>/Library/Application Support/Hyperspeedcube/hyperspeedcube.yaml`
  - Windows: `%APPDATA%\Hyperspeedcube\config\hyperspeedcube.yaml`

## [0.2.0] - 2022-02-12

### Added

- **Status bar**
  - Text indicating puzzle save/load and other events
  - Twist count [(QSTM, FTM, STM, ETM)](https://www.speedsolving.com/wiki/index.php/Metric)
- Automatic reloading of preferences file when modified externally
- Customizable keybinds for undo, redo, etc.
- Reset (<kbd>Ctrl</kbd>+<kbd>R</kbd>)
- New Rubik's 3D (<kbd>F3</kbd>)
- New Rubik's 4D (<kbd>F4</kbd>)

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
