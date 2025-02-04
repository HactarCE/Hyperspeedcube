# Changelog

All notable changes to Hyperspeedcube will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

[@Edan-Purple]: https://github.com/Edan-Purple
[@JosieElliston]: https://github.com/JosieElliston
[@Leoongithub]: https://github.com/Leoongithub
[@milojacquet]: https://github.com/milojacquet
[@picuber]: https://github.com/picuber
[@Sonicpineapple]: https://github.com/Sonicpineapple
[@thatcomputerguy0101]: https://github.com/thatcomputerguy0101

## [2.0.0-pre.12-dev] - 2024-05-31

### Added

- Light mode
  - Theme is automatically detected on supported OSes
  - Button in top right toggles theme

### Changed

- The program now starts maximized
- Rotations are no longer twists
- Renamed "Realign puzzle on release" to "Snap puzzle on release"
- 4D FOV now keeps W=0 plane fixed instead of W=1
- Menu buttons (file, edit, etc.) collapse into one menu button when there's not enough space (such as on mobile)
- Dynamic twist speed lasts until the queue is empty instead of slowing down as the queue gets shorter

### Removed

- Smart realign option (always enabled)

## [1.0.11] - 2025-01-05

### Fixed

- Cursor interaction on Safari ([@thatcomputerguy0101] [#69](https://github.com/HactarCE/Hyperspeedcube/pull/69))

## [1.0.10] - 2025-01-05

### Added

- Timer auto-start and auto-stop ([@JosieElliston] [#66](https://github.com/HactarCE/Hyperspeedcube/pull/66))

### Fixed

- Status bar revealing solved state when blindfold is on ([@JosieElliston] [#66](https://github.com/HactarCE/Hyperspeedcube/pull/66))

## [1.0.9] - 2024-11-20

### Fixed

- Crash when starting timer on web

## [1.0.8] - 2024-11-20

### Added

- Built-in timer ([@JosieElliston] [#65](https://github.com/HactarCE/Hyperspeedcube/pull/65))

### Fixed

- Glitchy window size on program start ([@thatcomputerguy0101] [#49](https://github.com/HactarCE/Hyperspeedcube/pull/49))
- macOS universal binary ([@thatcomputerguy0101] [#51](https://github.com/HactarCE/Hyperspeedcube/pull/51))

## [1.0.7] - 2024-06-06

### Fixed

- Crash due to keybinds reference font size ([#36](https://github.com/HactarCE/Hyperspeedcube/issues/36))

## [1.0.6] - 2023-10-18

### Added

- Fallback copy/paste on web ([@milojacquet] [#37](https://github.com/HactarCE/Hyperspeedcube/pull/37))

## [1.0.5] - 2023-01-21

### Changed

- Tweaked "Welcome" and "About" windows

### Fixed

- Incorrect key names on web
- Modifier keys sticking on web
- "Solved!" message not appearing most of the time

## [1.0.4] - 2023-01-21

### Added

- Web version
- Configurable FPS limit
- Copy/paste commands
  - File → Open from clipboard (<kbd>Ctrl</kbd>+<kbd>V</kbd>)
  - File → Copy (.hsc) (<kbd>Ctrl</kbd>+<kbd>C</kbd>)
  - File → Copy (.log) (<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>C</kbd>)
- "Click to copy" button on YAML editor

### Changed

- Tweaked UI styling due to egui update
- Disabled VSync

## [1.0.3] - 2023-01-05

### Added

- Opening log file passed via CLI arg

### Fixed

- Missing scrollbar on piece filters window

## [1.0.2] - 2022-12-16

### Added

- Built-in "Next" and "Previous" presets for "Filter" keybindings ([@Sonicpineapple])

### Changed

- Tweaked default keybinds ([@Sonicpineapple])

### Fixed

- Preferences not saving on macOS ([@Edan-Purple])
- Keys staying held after window loses focus
- Incorrect key names on Linux ([@picuber])
- Holding a key no longer twists the puzzle repeatedly ([@Edan-Purple])

## [1.0.1] - 2022-11-23

### Added

- "Clip 4D" option (enabled by default)
- "Realign on keypress" option (enabled by default)
- Keybinding command to select custom view presets (not bound by default)
- Easter egg

### Changed

- Optimized rendering of invisible pieces; large puzzles now render more smoothly when many pieces are hidden ([@Sonicpineapple])

### Fixed

- Incorrect YouTube link in welcome screen ([@Leoongithub])
- Hovered sticker with off-center puzzle
- Selection of fallback graphics adapter
- Keybinds reference resizing being janky

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
