use egui::NumExt;
use itertools::Itertools;
use key_names::KeyMappingCode;

use crate::app::App;
use crate::commands::{PuzzleCommand, SelectThing};
use crate::preferences::Key;

const KEY_PADDING: f32 = 0.05;

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    let prefs = app.prefs.gui.keybinds_reference;

    ui.scope(|ui| {
        let bg_fill = &mut ui.visuals_mut().widgets.noninteractive.bg_fill;
        let alpha = app.prefs.gui.keybinds_reference.opacity;
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

        let min_scale = ui.spacing().button_padding.y * 2.0 + ui.spacing().interact_size.y;

        if let Some(total_rect) = areas.iter().map(|area| area.rect).reduce(egui::Rect::union) {
            // How much space is available?
            let max_scale = ui.available_size() / total_rect.size();
            let scale = max_scale.min_elem().at_least(min_scale).floor();
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
                                    .shrink(KEY_PADDING * scale);
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
        let default_prefs = crate::preferences::DEFAULT_PREFS.gui.keybinds_reference;

        let mut changed = false;

        let r = ui.add(super::util::WidgetWithReset {
            label: "Opacity",
            value: &mut app.prefs.gui.keybinds_reference.opacity,
            reset_value: default_prefs.opacity,
            reset_value_str: format!("{:.0}%", default_prefs.opacity * 100.0,),
            make_widget: super::util::make_percent_drag_value,
        });
        changed |= r.changed();

        let r = ui.checkbox(
            &mut app.prefs.gui.keybinds_reference.function,
            "Function keys",
        );
        changed |= r.changed();

        let r = ui.checkbox(
            &mut app.prefs.gui.keybinds_reference.navigation,
            "Navigation keys",
        );
        changed |= r.changed();

        let r = ui.checkbox(&mut app.prefs.gui.keybinds_reference.numpad, "Numpad");
        changed |= r.changed();

        app.prefs.needs_save |= changed;
    });
}

fn draw_key(ui: &mut egui::Ui, app: &mut App, key: KeyMappingCode, rect: egui::Rect) {
    let matching_keybinds = app
        .resolve_keypress(&app.prefs.puzzle_keybinds[app.puzzle.ty()], Some(key), None)
        .into_iter()
        .take_while(|bind| bind.command != PuzzleCommand::None)
        .collect_vec();

    let s = matching_keybinds
        .first()
        .map(|bind| bind.command.short_description())
        .unwrap_or_default();
    // let s = get_key_name(key);

    let text = autosize_button_text(ui, s, rect.size());

    let mut button = egui::Button::new(text).sense(egui::Sense::hover());
    if app.pressed_keys().contains(&Key::Sc(key)) {
        // button = button.fill(egui::Color32::DARK_RED);
        button = button.stroke(ui.style().noninteractive().fg_stroke);
    }
    let r = ui.put(rect, button);
    if !matching_keybinds.is_empty() {
        r.on_hover_ui(|ui| {
            // Adjust spacing so we don't have to add spaces manually.
            let space_width = ui
                .fonts()
                .glyph_width(&egui::TextStyle::Body.resolve(ui.style()), ' ');
            ui.spacing_mut().item_spacing.x = space_width;

            for bind in matching_keybinds {
                ui.horizontal_wrapped(|ui| match &bind.command {
                    PuzzleCommand::Twist {
                        face,
                        direction,
                        layer_mask,
                    } => {
                        ui.label("Twist");
                        ui.strong(face.map(|f| f.name()).unwrap_or("selected"));
                        ui.label("face in");
                        ui.strong(direction.name());
                        ui.label("direction");
                        if !layer_mask.is_default() {
                            ui.label("(layer");
                            ui.strong(layer_mask.long_description());
                            ui.add_space(-space_width);
                            ui.label(")");
                        }
                    }
                    PuzzleCommand::Recenter { face } => {
                        ui.label("Recenter");
                        ui.strong(face.map(|f| f.name()).unwrap_or("selected"));
                        ui.label("face");
                    }

                    PuzzleCommand::HoldSelect(thing) | PuzzleCommand::ToggleSelect(thing) => {
                        ui.label(match &bind.command {
                            PuzzleCommand::HoldSelect(_) => "Hold select",
                            PuzzleCommand::ToggleSelect(_) => "Toggle select",
                            _ => unreachable!(),
                        });

                        match thing {
                            SelectThing::Face(f) => {
                                ui.strong(f.name());
                                ui.label("face");
                            }
                            SelectThing::Layers(l) => {
                                ui.label("layer");
                                ui.strong(l.long_description());
                            }
                            SelectThing::PieceType(p) => {
                                ui.strong(p.name());
                                ui.label("pieces");
                            }
                        }
                    }
                    PuzzleCommand::ClearToggleSelect(category) => {
                        ui.label(format!(
                            "Clear selected {}s",
                            category.to_string().to_ascii_lowercase(),
                        ));
                    }
                    PuzzleCommand::None => unreachable!(),
                });
            }
        });
    }
}

fn autosize_button_text(
    ui: &mut egui::Ui,
    button_text: String,
    button_size: egui::Vec2,
) -> egui::RichText {
    let max_size = button_size - ui.spacing().button_padding * 2.0;
    let mut text = egui::RichText::new(button_text);
    let mut font_size = egui::TextStyle::Button.resolve(ui.style()).size;
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
