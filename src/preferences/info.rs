use serde::{Deserialize, Serialize};

use crate::puzzle::TwistMetric;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct InfoPreferences {
    pub metric: TwistMetric,
}
