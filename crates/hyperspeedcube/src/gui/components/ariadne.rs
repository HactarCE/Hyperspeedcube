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
        let escape_code_str = &remaining[2..escape_end - 1];
        match escape_code_str {
            "0" => format = default_format.clone(),

            _ if let Ok(i) = escape_code_str.parse::<u8>()
                && (30..=37).contains(&i) =>
            {
                format.color = term_color_16(i - 30);
            }

            _ if let Ok(i) = escape_code_str.parse::<u8>()
                && (90..=97).contains(&i) =>
            {
                format.color = term_color_16(i - 90 + 8);
            }

            _ if let Some(color_index_str) = escape_code_str.strip_prefix("38;5;") => {
                match color_index_str.parse::<u8>() {
                    Ok(color_index) => format.color = themed(term_color_256(color_index)),
                    Err(e) => log::warn!("Unknown color code {e:?}"),
                }
            }

            _ => log::warn!("Unknown escape code {escape_code_str:?}"),
        }

        remaining = &remaining[escape_end..];
    }

    ui.add(egui::Label::new(text_job).wrap_mode(egui::TextWrapMode::Extend))
}

fn term_color_16(i: u8) -> egui::Color32 {
    // Base16 3024
    let color = [
        egui::hex_color!("#282a2e"),
        egui::hex_color!("#a54242"),
        egui::hex_color!("#8c9440"),
        egui::hex_color!("#de935f"),
        egui::hex_color!("#5f819d"),
        egui::hex_color!("#85678f"),
        egui::hex_color!("#5e8d87"),
        egui::hex_color!("#707880"),
        egui::hex_color!("#373b41"),
        egui::hex_color!("#cc6666"),
        egui::hex_color!("#b5bd68"),
        egui::hex_color!("#f0c674"),
        egui::hex_color!("#81a2be"),
        egui::hex_color!("#b294bb"),
        egui::hex_color!("#8abeb7"),
        egui::hex_color!("#c5c8c6"),
    ][i as usize % 16];

    if i < 8 {
        color.gamma_multiply_u8(240) // very slightly transparent
    } else {
        color
    }
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
