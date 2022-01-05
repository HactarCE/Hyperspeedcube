use imgui::*;
use key_names::KeyMappingCode;
use rfd::{FileDialog, MessageButtons, MessageDialog};
use std::fmt;
use std::sync::Mutex;
use winit::event::{ElementState, Event, ModifiersState, VirtualKeyCode};

use super::{util, AppState};
use crate::config::{Key, Keybind};

const KEYBIND_POPUP_TITLE: &str = "Press a key combination";

lazy_static! {
    static ref KEYBIND_POPUP_STATE: Mutex<KeybindPopupState> =
        Mutex::new(KeybindPopupState::default());
}

#[derive(Default)]
struct KeybindPopupState {
    open_this_frame: bool,
    is_open: bool,
    callback: Option<Box<dyn Send + FnOnce(Keybind)>>,

    keybind: Option<Keybind>,

    mods: ModifiersState,
    last_vk_pressed: Option<VirtualKeyCode>,
    last_sc_pressed: Option<KeyMappingCode>,

    use_vk: bool,
}
impl KeybindPopupState {
    fn update_keybind(&mut self) {
        let sc = self.last_sc_pressed.map(Key::Sc);
        let vk = self.last_vk_pressed.map(Key::Vk);
        let key = if self.use_vk { vk.or(sc) } else { sc.or(vk) };
        let command = self.keybind.take().unwrap_or_default().command;

        self.keybind = Some(Keybind::new(key, self.mods, command));
    }
    fn set_key(&mut self, sc: KeyMappingCode, vk: VirtualKeyCode) {
        self.last_sc_pressed = Some(sc);
        self.last_vk_pressed = Some(vk);
        self.update_keybind();
    }
    fn confirm(&mut self) {
        let keybind = self.keybind.take().unwrap_or_default();
        self.callback.take().expect("no keybind callback")(keybind);
        self.is_open = false;
    }
    fn cancel(&mut self) {
        self.is_open = false;
    }
}

pub(super) fn file_dialog() -> FileDialog {
    FileDialog::new()
        .add_filter("Magic Cube 4D Log Files", &["log"])
        .add_filter("All files", &["*"])
}
pub(super) fn error_dialog(title: &str, e: impl fmt::Display) {
    MessageDialog::new()
        .set_title(title)
        .set_description(&e.to_string())
        .show();
}
pub(super) fn confirm_discard_changes_dialog(action: &str) -> MessageDialog {
    MessageDialog::new()
        .set_title("Unsaved changes")
        .set_description(&format!("Discard puzzle state and {}?", action))
        .set_buttons(MessageButtons::YesNo)
}

pub(super) fn open_keybind_popup(
    old_keybind: Keybind,
    set_new_keybind_callback: impl 'static + Send + FnOnce(Keybind),
) {
    // We can't actually open the popup here; it has to be opened at the same
    // imgui stack level as where it's created. From the perspective of an
    // imgui-rs user, this is very confusing. See this issue:
    // https://github.com/ocornut/imgui/issues/1422
    // Also mention cross-platform compatibility issues.

    let mut popup = KEYBIND_POPUP_STATE.lock().unwrap();

    *popup = KeybindPopupState {
        open_this_frame: true,
        is_open: true,
        callback: Some(Box::new(set_new_keybind_callback)),

        keybind: Some(old_keybind),

        mods: ModifiersState::default(),
        last_vk_pressed: None,
        last_sc_pressed: None,

        use_vk: popup.use_vk,
    };
}
pub(super) fn build_keybind_popup(app: &mut AppState) {
    let mut popup = KEYBIND_POPUP_STATE.lock().unwrap();
    if !popup.is_open {
        return;
    }
    let popup = &mut *popup;
    let ui = app.ui;

    if popup.open_this_frame {
        ui.open_popup(KEYBIND_POPUP_TITLE);
        popup.open_this_frame = false;
    }

    unsafe {
        // Workaround for https://github.com/imgui-rs/imgui-rs/issues/201
        sys::igSetNextWindowSize(sys::ImVec2::new(500.0, 0.0), Condition::Always as i32);
        sys::igSetNextWindowPos(
            util::get_viewport_center(ui),
            Condition::Always as i32,
            sys::ImVec2::new(0.5, 0.5),
        );
    }

    PopupModal::new(&*KEYBIND_POPUP_TITLE)
        .resizable(false)
        .build(ui, || {
            let keybind = popup.keybind.clone().unwrap_or_default();
            if keybind.key.is_some() {
                ui.text(&keybind.to_string());
            } else {
                ui.text("(press a key)")
            }

            ui.spacing();

            if ui.button("OK") {
                popup.confirm();
            }
            ui.same_line();
            if ui.button("Cancel") {
                popup.cancel();
            }
            ui.same_line();
            util::text_with_inline_key_names(ui, "Press {Enter} to confirm or {Escape} to cancel.");
            ui.spacing();

            let compact_style = util::push_style_compact(ui);
            TreeNode::new("Advanced").build(ui, || {
                // TODO: tooltip explaining what this checkbox means and why
                // it's useful
                let mut use_sc = !popup.use_vk;
                if ui.checkbox(
                    "Use scancode (location-based) instead of virtual keycode",
                    &mut use_sc,
                ) {
                    popup.use_vk = !use_sc;
                    popup.update_keybind();
                }
                if ui.button("Bind Enter key") {
                    popup.set_key(KeyMappingCode::Enter, VirtualKeyCode::Return);
                }
                ui.same_line();
                if ui.button("Bind Escape key") {
                    popup.set_key(KeyMappingCode::Escape, VirtualKeyCode::Escape);
                }
            });
            drop(compact_style)
        });
}

/// Handles keyboard events for the keybind popup, if it is open. Returns `true`
/// if the event is consumed.
pub fn keybind_popup_handle_event(ev: &Event<()>) -> bool {
    let mut popup = KEYBIND_POPUP_STATE.lock().unwrap();

    popup.is_open
        && match ev {
            Event::WindowEvent { event, .. } => match event {
                winit::event::WindowEvent::KeyboardInput { input, .. }
                    if input.state == ElementState::Pressed =>
                {
                    match input.virtual_keycode {
                        Some(VirtualKeyCode::Return) if popup.mods.is_empty() => {
                            popup.confirm();
                            true
                        }
                        Some(VirtualKeyCode::Escape) if popup.mods.is_empty() => {
                            popup.cancel();
                            true
                        }
                        _ => {
                            popup.last_sc_pressed = key_names::sc_to_key(input.scancode as u16);
                            popup.last_vk_pressed = input.virtual_keycode;
                            popup.update_keybind();
                            true
                        }
                    }
                }
                winit::event::WindowEvent::ModifiersChanged(mods) => {
                    popup.mods = *mods;
                    false
                }
                _ => false,
            },
            _ => false,
        }
}
