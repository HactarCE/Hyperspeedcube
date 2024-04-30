//! Puzzle construction API usable by Rust code.
//!
//! These are all wrapped in `Arc<Mutex<T>>` so that the Lua API can access each
//! independently. These builders are a rare place where we accept mutable
//! aliasing in the Lua API, so the Rust API must also have mutable aliasing.

mod axis_system;
mod color_system;
mod naming_scheme;
mod ordering;
mod puzzle;
mod shape;
mod twist_system;

pub use axis_system::{AxisBuilder, AxisLayerBuilder, AxisSystemBuilder};
pub use color_system::{ColorBuilder, ColorSystemBuilder};
pub use naming_scheme::{BadName, NamingScheme};
pub use ordering::CustomOrdering;
pub use puzzle::{PieceBuilder, PuzzleBuilder};
pub use shape::ShapeBuilder;
pub use twist_system::{TwistBuilder, TwistSystemBuilder};

/// Iterates over elements names in canonical order, assigning unused
/// autogenerated names to unnamed elements.
pub fn iter_autonamed<'a, I: hypermath::IndexNewtype>(
    names: &'a NamingScheme<I>,
    order: impl 'a + IntoIterator<Item = I>,
    autonames: impl 'a + IntoIterator<Item = String>,
) -> impl 'a + Iterator<Item = (I, String)> {
    use std::collections::HashSet;

    let ids_to_names = names.ids_to_names();

    let used_names: HashSet<&String> = ids_to_names.values().collect();
    let mut unused_names = autonames
        .into_iter()
        .filter(move |s| !used_names.contains(&s));
    let mut next_unused_name = move || unused_names.next().expect("ran out of names");

    order.into_iter().map(move |id| {
        let a = match ids_to_names.get(&id) {
            Some(s) => s.to_owned(),
            None => next_unused_name(),
        };
        (id, a)
    })
}
