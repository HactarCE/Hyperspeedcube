# Changelog

All notable changes to Hyperspeedcube will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [UNRELEASED]

### Added

- "Clip 4D" option (enabled by default)
- Easter egg

### Changed

- Optimized rendering of invisible pieces; large puzzles now render more smoothly when many pieces are hidden

### Fixed

- Incorrect YouTube link in welcome screen
- Hovered sticker with off-center puzzle

## [1.0.0] - 2022-09-03

- Official release

## [0.9.2] - 2022-09-03

### Added

- Puzzle positioning settings in **View settings**
- Font size adjustment for keybinds reference
- Keybind sets whose names start with `^` are hidden from the **Keybind sets** tool

### Changed

- Keybind sets UI is now (hopefully) more intuitive
- Filter commands now use icons in keybinds reference

### Fixed

- Typo in welcome screen
- Loading of older MC4D log files with invalid scramble state
- Scancodes now use evdev instead of XKB on Linux

## [0.9.1] - 2022-08-30

### Added

- Welcome screen
- Built-in "Everything" preset for "Filter" keybindings

### Fixed

- MC4D-compatible log files are now saved correctly
- Edge twists now animate correctly

## [0.9.0] - 2022-08-28

### Added

- Preferences migration
- Keybinding sets
- Customizable mousebindings
- Editing keybinds, mousebindings, and presets as plaintext YAML
- Option to align puzzle when mouse is released (disabled by default)
- "Smart realign" option that automatically applies the appropriate rotation to the puzzle based on mouse drag rotation before controls that depend on face locations (enabled by default)
- Option to save hidden piece opacity in piece filter preset
- Keybinding command to select custom piece filters (not bound by default)
- Button to bind Numpad Enter key
- Roll setting

### Changed

- Tweaked piece filters UI
  - Clicking on a filter now applies the filter
  - Name of active filter is now highlighted
- Selected pieces are now unaffected by piece filters

### Fixed

- Rubik's 4D log file save/load
  - 180-degree face twists are now saved correctly
  - 1x1x1x1 non-face rotations are now parsed correctly

## [0.8.2] - 2022-08-13

### Fixed

- Puzzle not rerendering when twist animation is skipped
- Copy/paste mistakes in preferences UI
- Selecting a layer using "Puzzle controls" no longer deselects other layers
- Dropdowns at the bottom of the keybinds list are now accessible

## [0.8.1] - 2022-08-12

### Added

- Pressing enter after typing in a preset name now saves the preset

### Fixed

- Piece filter presets are no longer inverted

## [0.8.0] - 2022-08-12

### Added

- Piece filtering by color and type
  - Presets per puzzle
  - Current filters save with `.hsc` log file
- Marking pieces using left click while holding <kbd>Shift</kbd>
- View settings presets
- New system for specifying layer masks that changes with the number of layers in the puzzle
- Modifier key toggle buttons tool

### Changed

- Overhauled UI
  - Most windows are now floating
  - Reorganized preferences
  - Renamed "selection" to "grip"
- Combined "Grip axis" and "Grip layers" commands
- Combined selection fade duration and hover fade duration preferences
- Default 3D keybinds for <kbd>X</kbd>, <kbd>C</kbd>, <kbd>,</kbd>, and <kbd>.</kbd>

### Removed

- Option to highlight only sticker on hover instead of whole piece

## [0.7.3] - 2022-08-05

### Fixed

- Crash when scrambling a single-layer puzzle
- Preferences saving/loading now works again

## [0.7.2] - 2022-07-29 [YANKED]

## [0.7.1] - 2022-07-29

### Added

- Option to hide frontfaces and backfaces in 3D puzzles
- Temporarily adjusting view angle with mouse drag

### Changed

- Overhauled turn metrics
- Clicking outer stickers on 1x1x1x1 and 2x2x2x2 now executes 90-degree twists

### Known issues

- Preferences saving/loading is broken

## [0.7.0] - 2022-07-27

### Added

- All cubic puzzles from 1x1x1 to 9x9x9
- All hypercubic puzzles from 1x1x1x1 to 9x9x9x9
- 180-degree twists (no keybinds by default)
- Log file import/export for all puzzles
  - Use `.log` extension to save as MC4D-compatible log file
- QTM (Quarter Turn Metric) and ATM (Axial Turn Metric)
- Improved logging and fatal error handling

### Removed

- Filtering by piece type

### Fixed

- Twist metric descriptions and implementations are now more specific and more accurate

## [0.6.0] - 2022-06-02

### Added

- Scrambling (<kbd>Ctrl</kbd>+<kbd>F</kbd>)
- Clicking on a sticker to twist its face
  - Left click twists counterclockwise
  - Right click twists clockwise
  - Middle click recenters the face
  - Layer selections work as usual (e.g., <kbd>Shift</kbd> by default for wide moves)
- Solved state detection
- Animated opacity and outline changes
- Fine-grained outline settings
- Option to confirm discard only when the puzzle has been fully scrambled

### Changed

- Reorganized preferences

### Removed

- FPS limit setting

## [0.5.0] - 2022-05-29

### Added

- Interactive keybinds reference
- Puzzle controls panel

### Changed

- Improved battery usage when idling
- Improved outline rendering
- Improved 4D projection clipping at extreme values
- Tweaked default colors

### Removed

- Automatic reloading of preferences file when modified externally
- 2x and 8x MSAA settings

### Fixed

- Transparent stickers now have the correct color
- Stickers now render in the correct order
- Single keypress can no longer perform multiple twists at once

## [0.4.0] - 2022-03-15

### Added

- Blindfold mode (<kbd>Ctrl</kbd>+<kbd>B</kbd>)
- Customizable twist speed
- Lighting

### Changed

- Changed color format in preferences file
- Tweaked default colors
- Tweaked UI

## [0.3.1] - 2022-03-12

### Fixed

- Crash on startup on macOS
- Console window visible on Windows

## [0.3.0] - 2022-03-10

### Added

- Overhauled UI
  - Light/dark theme is now automatically detected from OS
  - Preferences are now docked to the left side of the window
  - Every preferences item has a reset button
  - Help tooltips
- Outline width setting
- Hidden stickers opacity setting
- About window

### Changed

- **Preferences are now saved in `hyperspeedcube.yaml` in the same directory as `hyperspeedcube.exe` by default.** To continue using the old location, create a file named `nonportable` (no file extension) in the same directory as `hyperspeedcube.exe`. The old preferences file can be found in the following locations:
  - Linux: `/home/<user>/.config/hyperspeedcube/hyperspeedcube.yaml`
  - macOS: `/Users/<user>/Library/Application Support/Hyperspeedcube/hyperspeedcube.yaml`
  - Windows: `%APPDATA%\Hyperspeedcube\config\hyperspeedcube.yaml`

### Fixed

- Dialogs hang on macOS
- Log file with no moves loads incorrectly

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
