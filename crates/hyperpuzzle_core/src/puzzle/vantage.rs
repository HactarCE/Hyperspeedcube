use std::collections::HashMap;

use super::{Axis, PerAxis, Twist};

pub struct VantageSet {
    name: String,
    axes: HashMap<String, Axis>,
    twist_directions: HashMap<String, PerAxis<Twist>>,
}
