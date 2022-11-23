use egui::NumExt;
use key_names::KeyMappingCode;

use super::Window;
use crate::app::App;
use crate::commands::{Command, PuzzleCommand};
use crate::gui::components::PrefsUi;
use crate::gui::util::{set_widget_spacing_to_space_width, subtract_space};
use crate::preferences::{Key, Keybind, DEFAULT_PREFS};
use crate::puzzle::{traits::*, LayerMask};

const SCALED_KEY_PADDING: f32 = 0.0;
const MIN_KEY_PADDING: f32 = 4.0;

pub(crate) const KEYBINDS_REFERENCE: Window = Window {
    name: "Keybinds reference",
    build,
    ..Window::DEFAULT
};

fn build(ui: &mut egui::Ui, app: &mut App) {
    ui.scope(|ui| {
        let prefs = app.prefs.info.keybinds_reference;

        let bg_fill = &mut ui.visuals_mut().widgets.noninteractive.bg_fill;
        let alpha = app.prefs.info.keybinds_reference.opacity;
        *bg_fill = bg_fill.linear_multiply(alpha);

        let mut areas = vec![MAIN_KEYS];

        if prefs.function {
            areas.push(FUNCTION_KEYS);
        }
        if prefs.navigation {
            areas.push(NAVIGATION_KEYS);
        }
        if prefs.function && prefs.navigation {
            areas.push(NAVIGATION_FUNCTION_KEYS);
        }
        if prefs.numpad {
            if prefs.navigation {
                areas.push(NUMPAD_KEYS);
            } else {
                areas.push(NUMPAD_KEYS_NO_NAV);
            }
        }

        let min_scale =
            ui.spacing().button_padding.y * 2.0 + ui.spacing().interact_size.y + MIN_KEY_PADDING;

        if let Some(total_rect) = areas.iter().map(|area| area.rect).reduce(egui::Rect::union) {
            // How much space is available?
            let max_scale = ui.available_size() / total_rect.size();
            let scale = max_scale.x.at_least(min_scale).round();
            // Allocate that much space.
            let (_id, rect) = ui.allocate_space(total_rect.size() * scale);
            let origin = rect.min - total_rect.min.to_vec2() * scale;
            for area in areas {
                let mut cursor = area.rect.min.to_vec2() * scale;
                for &row in area.rows {
                    for &element in row {
                        match element {
                            KeyboardElement::Key(key) => {
                                let key_size = get_key_size(key) * scale;
                                let key_rect = egui::Rect::from_min_size(origin + cursor, key_size)
                                    .shrink(MIN_KEY_PADDING + SCALED_KEY_PADDING * scale);
                                draw_key(ui, app, key, key_rect);
                                cursor.x += key_size.x;
                            }
                            KeyboardElement::Gap(dx) => cursor.x += dx * scale,
                        }
                    }

                    cursor.x = area.rect.left() * scale;
                    cursor.y += 1.0 * scale;
                }
            }
        }
    });

    ui.collapsing("Settings", |ui| {
        let mut changed = false;
        let mut prefs_ui = PrefsUi {
            ui,
            current: &mut app.prefs.info.keybinds_reference,
            defaults: &DEFAULT_PREFS.info.keybinds_reference,
            changed: &mut changed,
        };

        prefs_ui.percent("Opacity", access!(.opacity));
        prefs_ui.checkbox("Function keys", access!(.function));
        prefs_ui.checkbox("Navigation keys", access!(.navigation));
        prefs_ui.checkbox("Numpad", access!(.numpad));
        prefs_ui.float("Max font size", access!(.max_font_size), |dv| {
            dv.fixed_decimals(1).clamp_range(1.0..=3.0).speed(0.01)
        });

        app.prefs.needs_save |= changed;
    });
}

fn draw_key(ui: &mut egui::Ui, app: &mut App, key: KeyMappingCode, rect: egui::Rect) {
    let puzzle_type = app.puzzle.ty();

    let vk = key_names::key_to_winit_vkey(key);
    let matching_puzzle_keybinds: Vec<&Keybind<PuzzleCommand>> = app
        .resolve_keypress(
            app.prefs.puzzle_keybinds[puzzle_type].get_active_keybinds(),
            Some(key),
            vk,
        )
        .into_iter()
        .take_while(|bind| bind.command != PuzzleCommand::None)
        .collect();
    let matching_global_keybinds: Vec<&Keybind<Command>> = app
        .resolve_keypress(&app.prefs.global_keybinds, Some(key), vk)
        .into_iter()
        .take_while(|bind| bind.command != Command::None)
        .collect();

    let s = matching_puzzle_keybinds
        .iter()
        .find_map(|bind| {
            let mut c = bind.command.clone();
            match &mut c {
                // Don't show keybinds that depend on a grip when we don't have an
                // axis gripped.
                PuzzleCommand::Twist { axis, .. } | PuzzleCommand::Recenter { axis } => {
                    match app.gripped_twist_axis(axis.as_deref()) {
                        Ok(gripped_axis) => {
                            *axis = Some(puzzle_type.info(gripped_axis).name.to_string())
                        }
                        Err(_) => return None,
                    }
                }
                _ => (),
            }
            Some(c.short_description(puzzle_type))
        })
        .or_else(|| {
            matching_puzzle_keybinds
                .first()
                .map(|bind| bind.command.short_description(puzzle_type))
        })
        .or_else(|| {
            matching_global_keybinds
                .first()
                .map(|bind| bind.command.short_description())
        })
        .unwrap_or_default();

    let text = autosize_button_text(
        ui,
        s,
        rect.size(),
        app.prefs.info.keybinds_reference.max_font_size,
    );

    let mut button = egui::Button::new(text).sense(egui::Sense::hover());
    if app.pressed_keys().contains(&Key::Sc(key)) {
        button = button.fill(egui::Color32::DARK_GREEN);
        button = button.stroke(ui.style().noninteractive().fg_stroke);
    }
    let r = ui.put(rect, button);
    r.on_hover_ui(|ui| {
        ui.heading(get_key_name(key));

        // Adjust spacing so we don't have to add spaces manually.
        set_widget_spacing_to_space_width(ui);

        for bind in matching_puzzle_keybinds {
            ui.horizontal_wrapped(|ui| match &bind.command {
                PuzzleCommand::Grip { axis, layers } => {
                    ui.label("Grip");
                    if let Some(twist_axis) = axis {
                        ui.strong(twist_axis);
                    }
                    if !layers.is_default() {
                        let layers = layers.to_layer_mask(puzzle_type.layer_count());
                        ui.strong(layers.long_description());
                    }
                }

                PuzzleCommand::Twist {
                    axis,
                    direction,
                    layers,
                } => {
                    let layers = layers.to_layer_mask(puzzle_type.layer_count());
                    if layers == puzzle_type.all_layers() {
                        ui.label("Rotate");
                        ui.strong("whole puzzle");
                        ui.label("in");
                        ui.strong(direction);
                        ui.label("direction relative to");
                        ui.strong(axis.as_deref().unwrap_or("gripped"));
                        ui.label("axis");
                    } else if layers != LayerMask(0) {
                        ui.label("Twist");
                        ui.strong(axis.as_deref().unwrap_or("gripped"));
                        ui.label("in");
                        ui.strong(direction);
                        ui.label("direction");
                        if !layers.is_default() {
                            ui.label("(");
                            subtract_space(ui);
                            ui.strong(layers.long_description());
                            subtract_space(ui);
                            ui.label(")");
                        }
                    }
                }
                PuzzleCommand::Recenter { axis } => {
                    ui.label("Recenter");
                    ui.strong(axis.as_deref().unwrap_or("gripped"));
                    ui.label("axis");
                }

                PuzzleCommand::Filter { mode, filter_name } => {
                    ui.label(mode.as_ref());
                    ui.strong(filter_name);
                    ui.label("preset");
                }

                PuzzleCommand::KeybindSet { keybind_set_name } => {
                    ui.label("Switch to");
                    ui.strong(keybind_set_name);
                    ui.label("keybinds");
                }

                PuzzleCommand::None => unreachable!(),
            });
        }

        for bind in matching_global_keybinds {
            ui.horizontal_wrapped(|ui| match &bind.command {
                Command::Open => ui.label("Open"),
                Command::Save => ui.label("Save"),
                Command::SaveAs => ui.label("Save As"),
                Command::Exit => ui.label("Exit"),

                Command::Undo => ui.label("Undo"),
                Command::Redo => ui.label("Redo"),
                Command::Reset => ui.label("Reset"),

                Command::ScrambleN(n) => {
                    ui.label("Scramble");
                    ui.strong(n.to_string())
                }
                Command::ScrambleFull => ui.label("Scramble fully"),

                Command::NewPuzzle(ty) => {
                    ui.label("Load new");
                    ui.strong(ty.name());
                    ui.label("puzzle")
                }

                Command::ToggleBlindfold => ui.label("Toggle blindfold"),

                Command::None => unreachable!(),
            });
        }
    });
}

fn autosize_button_text(
    ui: &mut egui::Ui,
    button_text: String,
    button_size: egui::Vec2,
    max_font_size: f32,
) -> egui::RichText {
    let max_size = button_size - ui.spacing().button_padding * 2.0;
    let mut text = egui::RichText::new(button_text);
    let mut font_size = egui::TextStyle::Button.resolve(ui.style()).size * max_font_size;
    while font_size > 0.0 {
        text = text.size(font_size);
        let text_size = egui::WidgetText::RichText(text.clone())
            .into_galley(ui, Some(false), f32::INFINITY, egui::TextStyle::Button)
            .size();
        if text_size.x <= max_size.x && text_size.y <= max_size.y {
            return text;
        }
        font_size -= 1.0;
    }
    egui::RichText::new("")
}

macro_rules! keyboard_key {
    (NextRow) => {
        KeyboardElement::NextRow
    };
    ($key:ident) => {
        KeyboardElement::Key(KeyMappingCode::$key)
    };
    ($gap:expr) => {
        KeyboardElement::Gap($gap)
    };
}
macro_rules! keyboard_row {
    ($($element:tt)*) => { &[$(keyboard_key!($element)),*] };
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum KeyboardElement {
    Key(KeyMappingCode),
    Gap(f32),
}

#[derive(Debug, Copy, Clone)]
struct KeyboardArea {
    rect: egui::Rect,
    rows: &'static [&'static [KeyboardElement]],
}

const FUNCTION_KEYS: KeyboardArea = KeyboardArea {
    rect: egui::Rect {
        min: egui::pos2(0.0, 0.0),
        max: egui::pos2(15.0, 1.0),
    },
    rows: &[keyboard_row![Escape 1.0 F1 F2 F3 F4 0.5 F5 F6 F7 F8 0.5 F9 F10 F11 F12]],
};
const MAIN_KEYS: KeyboardArea = KeyboardArea {
    rect: egui::Rect {
        min: egui::pos2(0.0, 1.5),
        max: egui::pos2(15.0, 6.5),
    },
    rows: &[
        keyboard_row![Backquote Digit1 Digit2 Digit3 Digit4 Digit5 Digit6 Digit7 Digit8 Digit9 Digit0 Minus Equal Backspace],
        keyboard_row![Tab KeyQ KeyW KeyE KeyR KeyT KeyY KeyU KeyI KeyO KeyP BracketLeft BracketRight Backslash],
        keyboard_row![CapsLock KeyA KeyS KeyD KeyF KeyG KeyH KeyJ KeyK KeyL Semicolon Quote Enter],
        keyboard_row![ShiftLeft KeyZ KeyX KeyC KeyV KeyB KeyN KeyM Comma Period Slash ShiftRight],
        keyboard_row![ControlLeft MetaLeft AltLeft Space AltRight MetaRight ContextMenu ControlRight],
    ],
};
const NAVIGATION_KEYS: KeyboardArea = KeyboardArea {
    rect: egui::Rect {
        min: egui::pos2(15.25, 1.5),
        max: egui::pos2(18.25, 6.5),
    },
    rows: &[
        keyboard_row![Insert Home PageUp],
        keyboard_row![Delete End PageDown],
        keyboard_row![],
        keyboard_row![1.0 ArrowUp],
        keyboard_row![ArrowLeft ArrowDown ArrowRight],
    ],
};
const NAVIGATION_FUNCTION_KEYS: KeyboardArea = KeyboardArea {
    rect: egui::Rect {
        min: egui::pos2(15.25, 0.0),
        max: egui::pos2(18.25, 1.0),
    },
    rows: &[keyboard_row![PrintScreen ScrollLock Pause]],
};
const NUMPAD_KEYS: KeyboardArea = KeyboardArea {
    rect: egui::Rect {
        min: egui::pos2(18.5, 1.5),
        max: egui::pos2(22.5, 6.5),
    },
    rows: &[
        keyboard_row![NumLock NumpadDivide NumpadMultiply NumpadSubtract],
        keyboard_row![Numpad7 Numpad8 Numpad9 NumpadAdd],
        keyboard_row![Numpad4 Numpad5 Numpad6],
        keyboard_row![Numpad1 Numpad2 Numpad3 NumpadEnter],
        keyboard_row![Numpad0 NumpadDecimal],
    ],
};
const NUMPAD_KEYS_NO_NAV: KeyboardArea = KeyboardArea {
    rect: egui::Rect {
        min: egui::pos2(15.5, 1.5),
        max: egui::pos2(19.5, 6.5),
    },
    ..NUMPAD_KEYS
};

fn get_key_size(key: KeyMappingCode) -> egui::Vec2 {
    use KeyMappingCode::*;

    let w = match key {
        Backspace => 2.0,
        Tab | Backslash => 1.5,
        CapsLock => 1.75,
        Enter => 2.25,
        ShiftLeft => 2.25,
        ShiftRight => 2.75,
        ControlLeft | MetaLeft | AltLeft | AltRight | MetaRight | ContextMenu | ControlRight => {
            1.25
        }
        Space => 6.25,
        Numpad0 => 2.0,
        _ => 1.0,
    };

    let h = match key {
        NumpadAdd | NumpadEnter => 2.0,
        _ => 1.0,
    };

    egui::vec2(w, h)
}

fn get_key_name(key: KeyMappingCode) -> String {
    use KeyMappingCode::*;
    match key {
        // Home => todo!(),
        NumLock => "Num\nLock",
        // Pause => todo!(),
        ShiftLeft | ShiftRight => key_names::SHIFT_STR,
        ControlLeft | ControlRight => key_names::CTRL_STR,
        AltLeft | AltRight => key_names::ALT_STR,
        MetaLeft | MetaRight => {
            if cfg!(windows) {
                return egui::special_emojis::OS_WINDOWS.to_string();
            } else if cfg!(os = "macos") {
                "⌘"
            } else if cfg!(os = "linux") {
                return egui::special_emojis::OS_LINUX.to_string();
            } else {
                key_names::LOGO_STR
            }
        }
        ArrowDown => "⬇",
        ArrowLeft => "⬅",
        ArrowRight => "➡",
        ArrowUp => "⬆",
        Backspace => "Bksp",
        CapsLock => "Caps",
        ContextMenu => "🗖",
        Delete => "Del",
        Escape => "Esc",
        Insert => "Ins",
        Numpad0 => "0",
        Numpad1 => "1",
        Numpad2 => "2",
        Numpad3 => "3",
        Numpad4 => "4",
        Numpad5 => "5",
        Numpad6 => "6",
        Numpad7 => "7",
        Numpad8 => "8",
        Numpad9 => "9",
        NumpadAdd => "+",
        NumpadComma => ",",
        NumpadDecimal => ".",
        NumpadDivide => "/",
        NumpadEnter => "Enter",
        NumpadEqual => "=",
        NumpadMultiply => "*",
        NumpadParenLeft => "(",
        NumpadParenRight => ")",
        NumpadSubtract => "-",
        PageDown => "PgDn",
        PageUp => "PgUp",
        PrintScreen => "PrtSc",
        ScrollLock => "Scroll\nLock",
        Tab => "Tab",
        _ => return key_names::key_name(key),
    }
    .to_string()
}
