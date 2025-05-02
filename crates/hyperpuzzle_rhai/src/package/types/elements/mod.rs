use std::sync::Arc;

use hypermath::{IndexNewtype, Vector};
use hyperpuzzle_core::prelude::*;
use hyperpuzzle_impl_nd_euclid::builder::*;
use parking_lot::MappedMutexGuard;

mod color;

use super::*;

pub fn init_engine(engine: &mut Engine) {
    engine.register_type_with_name::<RhaiColor>("color");
    engine.register_type_with_name::<RhaiAxis>("axis");
    engine.register_type_with_name::<RhaiTwist>("twist");
}

pub fn register(module: &mut Module) {
    register_puzzle_element_type::<Color, ColorSystemBuilder>(module, "color");
    register_puzzle_element_type::<Axis, AxisSystemBuilder>(module, "axis");
    register_puzzle_element_type::<Twist, TwistSystemBuilder>(module, "twist");

    new_fn("to_string").set_into_module(module, |elem: &mut RhaiColor| -> Result<String> {
        Ok(match elem.lock_db()?.names.get(elem.id) {
            Some(name) => name.spec.clone(),
            None => "?".to_string(),
        })
    });
    new_fn("to_string").set_into_module(module, |elem: &mut RhaiAxis| -> Result<String> {
        Ok(match elem.lock_db()?.names.get(elem.id) {
            Some(name) => name.spec.clone(),
            None => "?".to_string(),
        })
    });
    new_fn("to_string").set_into_module(module, |elem: &mut RhaiTwist| -> Result<String> {
        Ok(match elem.lock_db()?.names.get(elem.id) {
            Some(name) => name.spec.clone(),
            None => "?".to_string(),
        })
    });

    FuncRegistration::new_getter("namespec").set_into_module(
        module,
        |axis: &mut RhaiAxis| -> Result<String> {
            Ok(axis
                .lock_db()?
                .names
                .get(axis.id)
                .ok_or("invalid axis")?
                .spec
                .clone())
        },
    );
    FuncRegistration::new_getter("name").set_into_module(
        module,
        |axis: &mut RhaiAxis| -> Result<String> {
            Ok(axis
                .lock_db()?
                .names
                .get(axis.id)
                .ok_or("invalid axis")?
                .preferred
                .clone())
        },
    );
    FuncRegistration::new_getter("vector")
        .set_into_module(module, |axis: &mut RhaiAxis| -> Result<Vector> {
            axis.vector()
        });

    color::register(module);
}

/// Rhai type for a color in a color system.
pub type RhaiColor = RhaiPuzzleElement<Color, ColorSystemBuilder>;
/// Rhai type for an axis in an axis system.
pub type RhaiAxis = RhaiPuzzleElement<Axis, AxisSystemBuilder>;
/// Rhai type for a twist in a twist system.
pub type RhaiTwist = RhaiPuzzleElement<Twist, TwistSystemBuilder>;

impl RhaiAxis {
    pub fn vector(&self) -> Result<hypermath::Vector> {
        Ok(self
            .db
            .lock()?
            .get(self.id)
            .map_err(|e| e.to_string())?
            .vector()
            .clone())
    }

    pub fn name_spec(&self) -> Result<Option<String>> {
        Ok(self
            .db
            .lock()?
            .names
            .get(self.id)
            .map(|name| name.spec.clone()))
    }
}

fn register_puzzle_element_type<I: IndexNewtype, DB: 'static>(
    module: &mut Module,
    name: &'static str,
) {
    new_fn("to_debug").set_into_module(module, move |elem: &mut RhaiPuzzleElement<I, DB>| {
        format!("{name}({})", elem.id)
    });

    new_fn("==").set_into_module(
        module,
        |c1: RhaiPuzzleElement<I, DB>, c2: RhaiPuzzleElement<I, DB>| c1 == c2,
    );
    new_fn("!=").set_into_module(
        module,
        |c1: RhaiPuzzleElement<I, DB>, c2: RhaiPuzzleElement<I, DB>| c1 != c2,
    );
}

/// Rhai handle to a puzzle element, indexed by ID.
pub struct RhaiPuzzleElement<I, DB> {
    /// ID of the puzzle element.
    pub id: I,
    /// Underlying database.
    pub db: Arc<dyn LockAs<DB>>,
}
impl<I: Clone, DB> Clone for RhaiPuzzleElement<I, DB> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            db: Arc::clone(&self.db),
        }
    }
}
impl<I: PartialEq, DB> PartialEq for RhaiPuzzleElement<I, DB> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id // just assume that DBs are equal
    }
}
impl<I: Eq, DB> Eq for RhaiPuzzleElement<I, DB> {}
impl<I, DB> RhaiPuzzleElement<I, DB> {
    pub fn lock_db(&self) -> Result<MappedMutexGuard<'_, DB>> {
        // TODO: infallible
        self.db.lock()
    }
}

pub trait LockAs<T>: Send + Sync {
    // TODO: infallible
    fn lock(&self) -> Result<MappedMutexGuard<'_, T>>;
}
