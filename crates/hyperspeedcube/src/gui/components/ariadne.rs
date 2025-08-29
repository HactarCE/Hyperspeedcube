use eframe::egui::{self, Color32};

pub fn show_ariadne_error_in_egui(ui: &mut egui::Ui, ansi_str: &str) -> egui::Response {
    let mut text_job = egui::text::LayoutJob::default();

    let themed = |color32: egui::Color32| {
        if ui.visuals().dark_mode {
            color32
        } else {
            let [r, g, b, _] = color32.to_array();
            Color32::from_rgb(r / 3 * 2, g / 3 * 2, b / 3 * 2)
        }
    };

    let mut remaining = ansi_str;
    let default_format =
        egui::TextFormat::simple(egui::FontId::monospace(14.0), ui.visuals().text_color());
    let mut format = default_format.clone();
    while !remaining.is_empty() {
        let escape_start = remaining.find("\x1b[").unwrap_or(remaining.len());
        let text = &remaining[..escape_start];
        if !text.is_empty() {
            text_job.append(text, 0.0, format.clone());
        }

        remaining = &remaining[escape_start..];
        if remaining.is_empty() {
            break;
        }
        let escape_end = remaining
            .find("m")
            .map(|i| i + 1)
            .unwrap_or(remaining.len());
        let escape_code = &remaining[2..escape_end - 1];
        match escape_code {
            "0" => format = default_format.clone(),
            "31" => format.color = ui.visuals().error_fg_color,
            "33" => format.color = ui.visuals().warn_fg_color,
            _ => {
                if let Some(color_index_str) = escape_code.strip_prefix("38;5;") {
                    match color_index_str.parse() {
                        Ok(color_index) => format.color = themed(term_color_256(color_index)),
                        Err(e) => log::warn!("unknown color code {e:?}"),
                    }
                } else {
                    log::warn!("unknown escape code {escape_code:?}");
                }
            }
        }

        remaining = &remaining[escape_end..];
    }

    ui.label(text_job)
}

fn term_color_256(i: u8) -> egui::Color32 {
    let [r, g, b] = if i < 16 {
        [
            [0x00, 0x00, 0x00],
            [0x80, 0x00, 0x00],
            [0x00, 0x80, 0x00],
            [0x80, 0x80, 0x00],
            [0x00, 0x00, 0x80],
            [0x80, 0x00, 0x80],
            [0x00, 0x80, 0x80],
            [0xc0, 0xc0, 0xc0],
            [0x80, 0x80, 0x80],
            [0xff, 0x00, 0x00],
            [0x00, 0xff, 0x00],
            [0xff, 0xff, 0x00],
            [0x00, 0x00, 0xff],
            [0xff, 0x00, 0xff],
            [0x00, 0xff, 0xff],
            [0xff, 0xff, 0xff],
        ][i as usize]
    } else if i >= 232 {
        [[
            0x08, 0x12, 0x1c, 0x26, 0x30, 0x3a, 0x44, 0x4e, 0x58, 0x60, 0x66, 0x76, 0x80, 0x8a,
            0x94, 0x9e, 0xa8, 0xb2, 0xbc, 0xc6, 0xd0, 0xda, 0xe4, 0xee,
        ][i as usize - 231]; 3]
    } else {
        let b = [0x00, 0x5f, 0x87, 0xaf, 0xd7, 0xff];
        let q = i as usize - 16;
        [b[q / 36], b[(q / 6) % 6], b[q % 6]]
    };
    egui::Color32::from_rgb(r, g, b)
}
