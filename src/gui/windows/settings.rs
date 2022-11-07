use super::{Location, Window, PREFS_WINDOW_WIDTH};
use crate::gui::components::prefs;

pub(crate) const APPEARANCE_SETTINGS: Window = Window {
    name: "Appearance",
    fixed_width: Some(PREFS_WINDOW_WIDTH),
    vscroll: true,
    build: |ui, app| {
        ui.collapsing("Colors", |ui| {
            prefs::build_colors_section(ui, app);
        });
        ui.collapsing("Outlines", |ui| {
            prefs::build_outlines_section(ui, app);
        });
        ui.collapsing("Opacity", |ui| {
            prefs::build_opacity_section(ui, app);
        });
        ui.collapsing("Performance", |ui| {
            prefs::build_graphics_section(ui, app);
        });
    },
    ..Window::DEFAULT
};

pub(crate) const INTERACTION_SETTINGS: Window = Window {
    name: "Interaction",
    fixed_width: Some(PREFS_WINDOW_WIDTH),
    build: prefs::build_interaction_section,
    ..Window::DEFAULT
};

pub(crate) const VIEW_SETTINGS: Window = Window {
    name: "View",
    location: Location::Floating,
    fixed_width: Some(PREFS_WINDOW_WIDTH),
    vscroll: true,
    build: prefs::build_view_section,
    cleanup: |_| (),
};
