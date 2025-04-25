//! Puzzle construction API usable by Rust code.
//!
//! These are all wrapped in `Arc<Mutex<T>>` so that the Lua API can access each
//! independently. These builders are a rare place where we accept mutable
//! aliasing in the Lua API, so the Rust API must also have mutable aliasing.

mod axis_layers;
mod axis_system;
mod color_system;
mod gizmos;
mod puzzle;
mod shape;
mod twist_system;
mod vantage_group;
mod vantage_set;

pub use axis_layers::{AxisLayerBuilder, AxisLayersBuilder};
use axis_system::AxisSystemBuildOutput;
pub use axis_system::{AxisBuilder, AxisSystemBuilder};
pub use color_system::{ColorBuilder, ColorSystemBuilder};
pub use puzzle::{PieceBuilder, PieceTypeBuilder, PuzzleBuilder};
pub use shape::ShapeBuilder;
pub use twist_system::{TwistBuilder, TwistSystemBuilder};
pub use vantage_group::VantageGroupBuilder;
pub use vantage_set::{
    AxisDirectionMapBuilder, RelativeAxisBuilder, RelativeTwistBuilder, VantageSetBuilder,
};
