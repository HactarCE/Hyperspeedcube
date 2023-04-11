use ndpuzzle::puzzle::jumbling::JumblingPuzzleSpec;
use parking_lot::Mutex;
use std::collections::BTreeMap;
use std::sync::Arc;

use super::PuzzleType;

lazy_static! {
    pub static ref PUZZLE_REGISTRY: Mutex<BTreeMap<String, Arc<PuzzleType>>> = {
        let mut ret = BTreeMap::new();
        ret.insert(
            "Default".to_string(),
            serde_yaml::from_str::<JumblingPuzzleSpec>(include_str!("default.yaml"))
                .expect("failed to build default puzzle")
                .build()
                .expect("failed to build default puzzle"),
        );
        Mutex::new(ret)
    };
}

// TODO: lazy puzzle init

// TODO: hide mutex & hashmap and expose a nicer API
