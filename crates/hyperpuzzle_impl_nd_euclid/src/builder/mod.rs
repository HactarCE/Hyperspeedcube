//! Puzzle construction API usable by Rust code.
//!
//! These are all wrapped in `Arc<Mutex<T>>` so that the Hyperpuzzlescript API
//! can access each independently. These builders are a rare place where we
//! accept mutable aliasing in the Hyperpuzzlescript API, so the Rust API must
//! also have mutable aliasing.

mod vantage_group;
mod vantage_set;

pub use vantage_group::VantageGroupBuilder;
pub use vantage_set::{
    AxisDirectionMapBuilder, RelativeAxisBuilder, RelativeTwistBuilder, VantageSetBuilder,
};
