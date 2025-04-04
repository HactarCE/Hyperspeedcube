use super::*;

mod blade;
mod vectors_and_points;

pub(super) fn register_all_types(module: &mut Module) {
    // blade::register(module);
    vectors_and_points::register(module);
}
