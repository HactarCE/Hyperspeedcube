use std::collections::HashMap;

use hypershape::{AbstractGroup, Group, IsometryGroup, PerGenerator, PerGroupElement};
use indexmap::IndexMap;

use super::vantage::VantageSet;
use super::{Axis, AxisInfo, PerAxis, PerGizmoFace, PerTwist, Twist, TwistInfo};

pub struct TwistSystem {
    pub id: String,
    pub name: String,

    /// List of axes, indexed by ID.
    pub axes: PerAxis<AxisInfo>,
    /// List of twists, indexed by ID.
    pub twists: PerTwist<TwistInfo>,

    /// Map from axis name to axis.
    pub axis_by_name: HashMap<String, Axis>,
    /// Map from twist name to twist.
    pub twist_by_name: HashMap<String, Twist>,

    /// Twist for each face of a twist gizmo.
    pub gizmo_twists: PerGizmoFace<Twist>,

    pub symmetry: IsometryGroup,
    pub axis_table: PerGenerator<PerAxis<Axis>>,
    pub twist_table: PerGenerator<PerTwist<Twist>>,

    pub vantage_sets: IndexMap<String, VantageSet>,
}
