use key_names::KeyMappingCode;
use std::sync::Arc;
use winit::event::{ElementState, ModifiersState, VirtualKeyCode, WindowEvent};

use super::keybinds_table::KeybindSet;
use super::util::ResponseExt;
use crate::app::App;
use crate::preferences::{Key, KeyCombo};

const KEYBIND_POPUP_SIZE: egui::Vec2 = egui::vec2(300.0, 200.0);

const SCANCODE_EXPLANATION: &str = "Scancodes are based on physical key position, while virtual keycodes depend on the keyboard layout";

#[derive(Default, Clone)]
pub(super) struct State {
    /// Callback to set the new key combo. This is `None` to indicate that the
    /// popup is closed.
    callback: Option<Arc<dyn Send + Sync + Fn(&mut App, KeyCombo)>>,

    key: Option<KeyCombo>,

    mods: ModifiersState,
    last_vk_pressed: Option<VirtualKeyCode>,
    last_sc_pressed: Option<KeyMappingCode>,

    use_vk: bool,
    use_vk_id: Option<egui::Id>,
}
impl State {
    fn update_keybind(&mut self) {
        let sc = self.last_sc_pressed.map(Key::Sc);
        let vk = self.last_vk_pressed.map(Key::Vk);
        let key = if self.use_vk { vk.or(sc) } else { sc.or(vk) };

        self.key = Some(KeyCombo::new(key, self.mods));
    }
    fn set_key(&mut self, sc: KeyMappingCode, vk: VirtualKeyCode) {
        self.last_sc_pressed = Some(sc);
        self.last_vk_pressed = Some(vk);
        self.update_keybind();
    }
    fn confirm(&mut self, app: &mut App) {
        if let Some(callback) = self.callback.take() {
            callback(app, self.key.unwrap_or_default());
        }
    }
    fn cancel(&mut self) {
        self.callback = None;
    }
}

pub(super) fn popup_state_mut(data: &mut egui::util::IdTypeMap) -> &mut State {
    data.get_temp_mut_or_default(popup_state_id())
}
pub(super) fn popup_state(ctx: &egui::Context) -> State {
    ctx.data().get_temp(popup_state_id()).unwrap_or_default()
}
fn popup_state_id() -> egui::Id {
    unique_id!()
}

pub(super) fn open<S: KeybindSet>(
    ctx: &egui::Context,
    key: Option<KeyCombo>,
    keybind_set: S,
    idx: usize,
) {
    let mut data = ctx.data();

    // General keybinds should use virtual keycodes by default, while puzzle
    // keybinds should use scancodes by default. If the user manually overrides
    // one, remember that decision for as long as the application is running.
    let use_vk_id = unique_id!().with(S::USE_VK_BY_DEFAULT);
    let use_vk = data.get_temp(use_vk_id).unwrap_or(S::USE_VK_BY_DEFAULT);

    *popup_state_mut(&mut data) = State {
        callback: Some(Arc::new(move |app, new_key_combo| {
            keybind_set.get_mut(&mut app.prefs)[idx].key = new_key_combo;
            app.prefs.needs_save = true;
        })),

        key,

        mods: ModifiersState::empty(),
        last_vk_pressed: None,
        last_sc_pressed: None,

        use_vk,
        use_vk_id: Some(use_vk_id),
    };
}

pub(super) fn build(ctx: &egui::Context, app: &mut App) -> Option<egui::Response> {
    #[allow(clippy::question_mark)]
    if popup_state(ctx).callback.is_none() {
        return None;
    }

    popup_state_mut(&mut ctx.data()).mods = app.pressed_modifiers();

    let r = egui::Area::new("keybind_popup")
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(ui.visuals().window_fill())
                .rounding(ui.visuals().window_rounding)
                .shadow(ui.visuals().window_shadow)
                .stroke(ui.visuals().window_stroke())
                .margin(egui::style::Margin::same(20.0))
                .show(ui, |ui| {
                    ui.allocate_ui_with_layout(
                        KEYBIND_POPUP_SIZE,
                        egui::Layout::top_down_justified(egui::Align::LEFT),
                        |ui| {
                            ui.vertical_centered(|ui| {
                                ui.spacing_mut().item_spacing.y = 20.0;

                                ui.heading("Press a key combination");

                                let key_combo = popup_state(ctx).key.unwrap_or_default();
                                if key_combo.key().is_some() {
                                    ui.strong(key_combo.to_string());
                                } else {
                                    ui.strong("(press a key)");
                                }

                                ui.columns(2, |columns| {
                                    let r = columns[0].with_layout(
                                        egui::Layout::top_down(egui::Align::RIGHT),
                                        |ui| ui.add_sized([60.0, 30.0], egui::Button::new("OK")),
                                    );
                                    if r.inner.clicked() {
                                        popup_state_mut(&mut ctx.data()).confirm(app);
                                    }

                                    let r = columns[1].with_layout(
                                        egui::Layout::top_down(egui::Align::LEFT),
                                        |ui| {
                                            ui.add_sized([60.0, 30.0], egui::Button::new("Cancel"))
                                        },
                                    );
                                    if r.inner.clicked() {
                                        popup_state_mut(&mut ctx.data()).cancel();
                                    }
                                });

                                ui.separator();

                                let mut use_vk = popup_state(ctx).use_vk;
                                let mut changed = false;
                                ui.horizontal(|ui| {
                                    ui.label("Key type:");
                                    let r = ui.selectable_value(&mut use_vk, false, "Scancode");
                                    changed |= r.changed();
                                    let r = ui.selectable_value(&mut use_vk, true, "Keycode");
                                    changed |= r.changed();
                                })
                                .response
                                .on_hover_explanation("", SCANCODE_EXPLANATION);
                                if changed {
                                    let mut data = ctx.data();
                                    let popup = popup_state_mut(&mut data);
                                    let use_vk_id = popup.use_vk_id;
                                    popup.use_vk = use_vk;
                                    popup.update_keybind();
                                    if let Some(id) = use_vk_id {
                                        data.insert_temp(id, use_vk);
                                    }
                                }

                                ui.horizontal(|ui| {
                                    if ui.button("Bind Enter key").clicked() {
                                        popup_state_mut(&mut ctx.data())
                                            .set_key(KeyMappingCode::Enter, VirtualKeyCode::Return);
                                    }
                                    if ui.button("Bind Escape key").clicked() {
                                        popup_state_mut(&mut ctx.data()).set_key(
                                            KeyMappingCode::Escape,
                                            VirtualKeyCode::Escape,
                                        );
                                    }
                                });
                            });
                        },
                    );
                });
        });

    Some(r.response)
}

/// Returns `true` if the key combo popup should handle the event exclusively.
/// Always call `key_combo_popup_handle_event()`, even if this function returns
/// `false`.
pub(crate) fn key_combo_popup_captures_event(ctx: &egui::Context, event: &WindowEvent) -> bool {
    let mut data = ctx.data();
    let popup = popup_state_mut(&mut data);

    popup.callback.is_some() && matches!(event, WindowEvent::KeyboardInput { .. })
}

/// Handles keyboard events for the keybind popup, if it is open. Returns `true`
/// if the event is consumed.
pub(crate) fn key_combo_popup_handle_event(
    ctx: &egui::Context,
    app: &mut App,
    event: &WindowEvent,
) {
    let mut data = ctx.data();
    let popup = popup_state_mut(&mut data);

    if popup.callback.is_some() {
        match event {
            winit::event::WindowEvent::KeyboardInput { input, .. }
                if input.state == ElementState::Pressed =>
            {
                match input.virtual_keycode {
                    Some(VirtualKeyCode::Return) if popup.mods.is_empty() => popup.confirm(app),
                    Some(VirtualKeyCode::Escape) if popup.mods.is_empty() => popup.cancel(),
                    _ => {
                        popup.last_sc_pressed = key_names::sc_to_key(input.scancode as u16);
                        popup.last_vk_pressed = input.virtual_keycode;
                        popup.update_keybind();
                    }
                }
            }

            winit::event::WindowEvent::ModifiersChanged(mods) => popup.mods = *mods,

            _ => (),
        }
    }
}
