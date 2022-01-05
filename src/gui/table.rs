use imgui::{sys::cty, ImString};

#[derive(Debug, Default, Copy, Clone)]
pub struct Column<'a> {
    pub label: &'a str,
    pub flags: imgui::sys::ImGuiTableColumnFlags_,
    pub init_width_or_weight: f32,
}
impl<'a> Column<'a> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            ..Self::default()
        }
    }
    pub fn flags(mut self, flags: imgui::sys::ImGuiTableColumnFlags_) -> Self {
        self.flags = flags;
        self
    }
}

#[must_use]
pub fn begin(str_id: &str, flags: imgui::sys::ImGuiTableFlags_, columns: &[Column<'_>]) -> bool {
    let s = ImString::new(str_id);
    let show_table = unsafe {
        imgui::sys::igBeginTable(
            s.as_ptr(),
            columns.len() as cty::c_int,
            flags as imgui::sys::ImGuiTableFlags,
            imgui::sys::ImVec2 { x: 0.0, y: 0.0 },
            0.0,
        )
    };
    if show_table {
        for c in columns {
            let s = ImString::new(c.label);
            unsafe {
                imgui::sys::igTableSetupColumn(
                    s.as_ptr(),
                    c.flags as imgui::sys::ImGuiTableColumnFlags,
                    c.init_width_or_weight,
                    0,
                );
            }
        }
    }
    show_table
}

pub fn scroll_freeze(cols: usize, rows: usize) {
    unsafe { imgui::sys::igTableSetupScrollFreeze(cols as cty::c_int, rows as cty::c_int) };
}

pub fn headers_row() {
    unsafe { imgui::sys::igTableHeadersRow() };
}

pub fn next_row() {
    unsafe { imgui::sys::igTableNextRow(0, 0.0) };
}
pub fn next_column() {
    unsafe { imgui::sys::igTableNextColumn() };
}

pub fn end() {
    unsafe { imgui::sys::igEndTable() };
}
