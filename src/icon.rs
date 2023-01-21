use winit::window::Icon;

const ICON_32_DATA: &[u8] = include_bytes!("../resources/icon/hyperspeedcube_32x32.png");

pub(crate) fn load_application_icon() -> Option<Icon> {
    match png::Decoder::new(ICON_32_DATA).read_info() {
        Ok(mut reader) => match reader.output_color_type() {
            (png::ColorType::Rgba, png::BitDepth::Eight) => {
                let mut img_data = vec![0_u8; reader.output_buffer_size()];
                if let Err(err) = reader.next_frame(&mut img_data) {
                    log::warn!("Failed to read icon data: {:?}", err);
                    return None;
                };
                let info = reader.info();
                match Icon::from_rgba(img_data, info.width, info.height) {
                    Ok(icon) => Some(icon),
                    Err(err) => {
                        log::warn!("Failed to construct icon: {:?}", err);
                        None
                    }
                }
            }
            other => {
                log::warn!(
                    "Failed to load icon data due to unknown color format: {:?}",
                    other,
                );
                None
            }
        },
        Err(err) => {
            log::warn!("Failed to load icon data: {:?}", err);
            None
        }
    }
}
