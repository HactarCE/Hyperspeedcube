use glium::glutin::event_loop::EventLoop;
use send_wrapper::SendWrapper;
use std::cell::RefCell;
use winit::window::Icon;


pub(crate) fn init<T>() -> EventLoop<T> {
    use glium::glutin::window::WindowBuilder;
    use glium::glutin::ContextBuilder;

    let wb = WindowBuilder::new()
        .with_title(crate::TITLE)
        .with_window_icon(load_application_icon());
    let cb = ContextBuilder::new()
        .with_vsync(false)
        .with_multisampling(0);

    let event_loop = EventLoop::with_user_event();

    DISPLAY.init(glium::Display::new(wb, cb, &event_loop).expect("failed to initialize display"));

    event_loop
}

lazy_static! {
    pub(crate) static ref DISPLAY: DisplayWrapper = DisplayWrapper::default();
}

pub(crate) struct DisplayWrapper(SendWrapper<RefCell<Option<&'static glium::Display>>>);
impl DisplayWrapper {
    fn init(&self, display: glium::Display) {
        *self.0.borrow_mut() = Some(Box::leak(Box::new(display)));
    }
}
impl Default for DisplayWrapper {
    fn default() -> Self {
        Self(SendWrapper::new(RefCell::new(None)))
    }
}
impl std::ops::Deref for DisplayWrapper {
    type Target = glium::Display;

    fn deref(&self) -> &Self::Target {
        RefCell::borrow(&self.0).unwrap()
    }
}

fn load_application_icon() -> Option<Icon> {
    match png::Decoder::new(crate::ICON_32).read_info() {
        Ok(mut reader) => match reader.output_color_type() {
            (png::ColorType::Rgba, png::BitDepth::Eight) => {
                let mut img_data = vec![0_u8; reader.output_buffer_size()];
                if let Err(err) = reader.next_frame(&mut img_data) {
                    eprintln!("Failed to read icon data: {:?}", err);
                    return None;
                };
                let info = reader.info();
                match Icon::from_rgba(img_data, info.width, info.height) {
                    Ok(icon) => Some(icon),
                    Err(err) => {
                        eprintln!("Failed to construct icon: {:?}", err);
                        None
                    }
                }
            }
            other => {
                eprintln!(
                    "Failed to load icon data due to unknown color format: {:?}",
                    other,
                );
                None
            }
        },
        Err(err) => {
            eprintln!("Failed to load icon data: {:?}", err);
            None
        }
    }
}
