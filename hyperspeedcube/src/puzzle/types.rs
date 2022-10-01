use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

use super::PuzzleType;

lazy_static! {
    pub static ref PUZZLE_REGISTRY: Mutex<HashMap<String, Arc<PuzzleType>>> = {
        let mut ret = HashMap::new();
        for i in 1..=9 {
            for ty in [
                super::rubiks_3d::puzzle_type(i),
                super::rubiks_4d::puzzle_type(i),
            ] {
                ret.insert(ty.name.clone(), ty);
            }
        }
        Mutex::new(ret)
    };
}

// TODO: lazy puzzle init

// TODO: hide mutex & hashmap and expose a nicer API
