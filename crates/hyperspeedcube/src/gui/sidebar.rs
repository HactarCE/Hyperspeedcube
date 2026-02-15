use std::collections::HashSet;

use egui::AtomExt;

use crate::L;
use crate::gui::components::PrefsUi;
use crate::gui::markdown::md;
use crate::gui::tabs::UtilityTab;
use crate::gui::util::{text_width, text_width_ctx};
use crate::gui::{App, AppUi, Tab};

const ICON_SIZE: f32 = 24.0;
const FONT_SIZE: f32 = 15.0;
const PADDING: f32 = 12.0;
const ITEM_SPACING: egui::Vec2 = egui::vec2(12.0, 12.0);
const ITEM_HEIGHT: f32 = 24.0;
const CHEVRON_SIZE: f32 = 24.0;

const SIDEBAR_ITEMS: &[SidebarItem] = &[
    SidebarItem::Tab(UtilityTab::Catalog),
    SidebarItem::Separator,
    SidebarItem::Tab(UtilityTab::PieceFilters),
    // SidebarItem::Tab(UtilityTab::Macros),
    // SidebarItem::Tab(UtilityTab::MoveInput),
    SidebarItem::Separator,
    SidebarItem::Tab(UtilityTab::Colors),
    SidebarItem::Tab(UtilityTab::Styles),
    SidebarItem::Tab(UtilityTab::View),
    SidebarItem::Tab(UtilityTab::Animation),
    SidebarItem::Separator,
    SidebarItem::Tab(UtilityTab::Interaction),
    // SidebarItem::Tab(UtilityTab::Keybinds),
    // SidebarItem::Tab(UtilityTab::Mousebinds),
    // SidebarItem::Separator,
    // SidebarItem::Tab(UtilityTab::Timer),
    // SidebarItem::Tab(UtilityTab::KeybindsReference),
    SidebarItem::Separator,
    // SidebarItem::Tab(UtilityTab::Timeline),
    // SidebarItem::Tab(UtilityTab::Scrambler),
    SidebarItem::Tab(UtilityTab::ImageGenerator),
    SidebarItem::Separator,
    SidebarItem::Tab(UtilityTab::PuzzleInfo),
    SidebarItem::Tab(UtilityTab::HpsLogs),
    SidebarItem::Tab(UtilityTab::DevTools),
    SidebarItem::Separator,
    #[cfg(debug_assertions)]
    SidebarItem::Tab(UtilityTab::Debug),
    SidebarItem::Tab(UtilityTab::About),
];

pub fn show(app_ui: &mut AppUi, ctx: &egui::Context) {
    let max_text_width = SIDEBAR_ITEMS
        .iter()
        .map(|item| item.min_width(ctx))
        .max_by(f32::total_cmp)
        .unwrap_or(0.0)
        + PADDING;

    let width_without_names = ICON_SIZE + PADDING * 2.0;

    let show_labels_anim = ctx.animate_bool(unique_id!(), app_ui.app.prefs.sidebar.show_labels);
    let show_labels = show_labels_anim == 1.0;

    let sidebar_width = width_without_names + show_labels_anim * max_text_width;

    let docked_tabs: HashSet<UtilityTab> = app_ui
        .dock_state
        .iter_all_tabs()
        .filter_map(|(_, t)| t.utility_tab())
        .collect();
    let visible_tabs: HashSet<UtilityTab> = app_ui
        .dock_state
        .iter_leaves()
        .filter_map(|(_s, leaf)| leaf.tabs.get(leaf.active.0)?.utility_tab())
        .collect();

    egui::SidePanel::left("sidebar")
        .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(0.0))
        .exact_width(sidebar_width + 1.0) // not sure why +1 pixel is needed when collapsed
        .resizable(false)
        .show_animated(ctx, app_ui.app.prefs.sidebar.show, |ui| {
            let spacing = ui.spacing_mut();
            spacing.item_spacing = ITEM_SPACING;
            spacing.button_padding.x = spacing.button_padding.y;
            spacing.scroll = egui::style::ScrollStyle::solid();
            spacing.scroll.floating = true;
            spacing.scroll.floating_width = spacing.scroll.bar_width;
            spacing.scroll.dormant_handle_opacity = 0.0;
            spacing.scroll.dormant_background_opacity = 0.0;

            let frame = egui::Frame::new().inner_margin(PADDING);

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                frame.show(ui, |ui| {
                    let prefs = &mut app_ui.app.prefs;
                    let icon = if prefs.sidebar.show_labels {
                        mdi!(CHEVRON_LEFT)
                    } else {
                        mdi!(CHEVRON_RIGHT)
                    };
                    if ui
                        .button(icon.atom_size(egui::vec2(CHEVRON_SIZE, CHEVRON_SIZE)))
                        .on_hover_text(if prefs.sidebar.show_labels {
                            L.sidebar.hide_labels
                        } else {
                            L.sidebar.show_labels
                        })
                        .clicked()
                    {
                        prefs.sidebar.show_labels ^= true;
                        prefs.needs_save = true;
                    }
                });

                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            frame.show(ui, |ui| {
                                for item in SIDEBAR_ITEMS {
                                    item.show(
                                        ui,
                                        app_ui,
                                        show_labels_anim > 0.0,
                                        &docked_tabs,
                                        &visible_tabs,
                                    );
                                }
                            });
                        });
                });
            });
        });
}

#[derive(Debug)]
enum SidebarItem {
    Tab(UtilityTab),
    Separator,
}

impl SidebarItem {
    fn min_width(&self, ctx: &egui::Context) -> f32 {
        match self {
            SidebarItem::Tab(tab) => {
                text_width_ctx(ctx, egui::RichText::from(tab.title()).size(FONT_SIZE))
            }
            SidebarItem::Separator => 0.0,
        }
    }

    fn show(
        &self,
        ui: &mut egui::Ui,
        app_ui: &mut AppUi,
        show_labels: bool,
        docked_tabs: &HashSet<UtilityTab>,
        visible_tabs: &HashSet<UtilityTab>,
    ) {
        match self {
            SidebarItem::Tab(tab) => {
                ui.horizontal(|ui| {
                    ui.set_max_width(ui.available_width());
                    ui.set_height(ITEM_HEIGHT);
                    let mut r = ui.interact(
                        ui.available_rect_before_wrap().expand2(ITEM_SPACING / 2.0),
                        unique_id!(tab),
                        egui::Sense::click(),
                    );
                    if docked_tabs.contains(tab) {
                        ui.painter().rect(
                            r.rect.shrink(1.0),
                            2.0,
                            ui.visuals().faint_bg_color,
                            (1.5, ui.visuals().weak_text_color()),
                            egui::StrokeKind::Inside,
                        );
                    }
                    let color = if app_ui.sidebar_utility == *tab && app_ui.is_sidebar_open
                        || r.is_pointer_button_down_on()
                        || r.clicked()
                        || visible_tabs.contains(tab)
                    {
                        ui.visuals().strong_text_color()
                    } else if r.hovered() || docked_tabs.contains(tab) {
                        ui.visuals().text_color()
                    } else {
                        egui::Color32::from_rgb(102, 102, 102)
                    };

                    r = r.on_hover_ui(|ui| {
                        if !app_ui.app.prefs.sidebar.show_labels {
                            ui.label(egui::RichText::from(tab.title()).size(FONT_SIZE));
                            ui.separator();
                        }
                        let mut left_click = &L.click_to.open_in_sidebar;
                        let mut right_click = &L.click_to.open_in_docked_tab;
                        if app_ui.app.prefs.interaction.swap_sidebar_mouse_buttons {
                            std::mem::swap(&mut left_click, &mut right_click);
                        }
                        md(ui, left_click.with(L.inputs.click));
                        md(ui, right_click.with(L.inputs.right_click));
                        md(ui, L.click_to.close.with(L.inputs.middle_click));
                    });

                    ui.add(
                        tab.icon()
                            .fit_to_exact_size(egui::vec2(ICON_SIZE, ICON_SIZE))
                            .tint(color),
                    );
                    if show_labels {
                        // Paint text directly to avoid allocating too much width
                        ui.painter().text(
                            ui.cursor().left_center(),
                            egui::Align2::LEFT_CENTER,
                            tab.title(),
                            egui::FontId::proportional(FONT_SIZE),
                            color,
                        );
                    }

                    let mods = ui.input(|input| input.modifiers);
                    let swap_clicks = app_ui.app.prefs.interaction.swap_sidebar_mouse_buttons;
                    let mut clicked_primary = r.clicked() && !mods.alt;
                    let mut clicked_secondary = r.secondary_clicked() && !mods.alt;
                    if swap_clicks {
                        std::mem::swap(&mut clicked_primary, &mut clicked_secondary);
                    }
                    if clicked_primary {
                        if docked_tabs.contains(tab) {
                            app_ui.activate_docked_utility(*tab);
                        } else {
                            app_ui.toggle_sidebar_utility(*tab);
                        }
                    } else if clicked_secondary {
                        app_ui.toggle_docked_utility(*tab);
                    } else if r.middle_clicked() || r.clicked() && mods.alt {
                        app_ui.close_utility(*tab);
                    }
                });
            }
            SidebarItem::Separator => {
                ui.separator();
            }
        }
    }
}
