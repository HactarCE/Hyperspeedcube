use imgui::*;

pub fn get_viewport(_ui: &Ui<'_>) -> sys::ImGuiViewport {
    unsafe { *imgui::sys::igGetMainViewport() }
}
pub fn get_viewport_center(ui: &Ui<'_>) -> imgui::sys::ImVec2 {
    let viewport = get_viewport(ui);
    let work_pos = viewport.WorkPos;
    let work_size = viewport.WorkSize;
    imgui::sys::ImVec2::new(
        work_pos.x + work_size.x / 2.0,
        work_pos.y + work_size.y / 2.0,
    )
}

/// Displays text with inline key names. E.g. "Press {Enter} to confirm."
pub fn text_with_inline_key_names(ui: &Ui<'_>, mut s: &str) {
    let _style_stack_tokens = (
        ui.push_style_var(StyleVar::FrameRounding(3.0)),
        ui.push_style_var(StyleVar::FrameBorderSize(1.0)),
        ui.push_style_var(StyleVar::ItemSpacing([2.0, 4.0])),
        ui.push_style_color(StyleColor::Button, [0.0, 0.0, 0.0, 0.0]),
        ui.push_style_color(StyleColor::ButtonActive, [0.0, 0.0, 0.0, 0.0]),
        ui.push_style_color(StyleColor::ButtonHovered, [0.0, 0.0, 0.0, 0.0]),
        ui.push_style_color(StyleColor::Border, ui.style_color(StyleColor::Text)),
    );
    loop {
        if let Some((normal, remaining)) = s.split_once("{") {
            if let Some((key_name, remaining)) = remaining.split_once("}") {
                ui.text(normal);
                ui.same_line();
                ui.small_button(key_name);
                ui.same_line();
                s = remaining;
                continue;
            }
        }
        ui.text(s);
        break;
    }
}

pub fn push_style_compact<'ui>(ui: &'ui Ui<'ui>) -> impl 'ui + Sized {
    let style = ui.clone_style();
    let [fp_x, fp_y] = style.frame_padding;
    let [is_x, is_y] = style.item_spacing;
    (
        ui.push_style_var(StyleVar::FramePadding([fp_x, (fp_y * 0.60).floor()])),
        ui.push_style_var(StyleVar::ItemSpacing([is_x, (is_y * 0.60).floor()])),
    )
}
