use crate::DISPLAY;
use glium::{program, Program};
use send_wrapper::SendWrapper;

macro_rules! load_shader {
    ($name:expr, $version:expr, srgb = $srgb:expr) => {
        {
            SendWrapper::new(program!(
                    &**DISPLAY,
                    $version => {
                        vertex: include_str!(concat!(stringify!($name), ".vert")),
                        fragment: include_str!(concat!(stringify!($name), ".frag")),
                        outputs_srgb: $srgb,
                    },
                ).expect(&format!("failed to compile '{}' shader in {}", stringify!($name), std::module_path!()))
            )
        }
    };
}

lazy_static! {
    pub static ref BASIC: SendWrapper<Program> = load_shader!(basic, 140, srgb = false);
    pub static ref OUTLINED: SendWrapper<Program> = load_shader!(outlined, 140, srgb = false);
}
