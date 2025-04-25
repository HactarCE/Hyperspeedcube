use std::sync::Arc;

use hyperpuzzle_impl_nd_euclid::builder::*;
use parking_lot::MappedMutexGuard;

use super::twist_system::RhaiTwistSystemBuilder;
use super::*;
use crate::package::types::elements::{LockAs, RhaiAxis};

pub fn init_engine(engine: &mut Engine) {
    engine.register_type_with_name::<RhaiAxisSystem>("axissystem");
}

pub fn register(module: &mut Module) {
    FuncRegistration::new_index_getter().set_into_module(
        module,
        |axis_system: &mut RhaiAxisSystem, name: String| -> Result<RhaiAxis> {
            let opt_id = axis_system.lock()?.names.id_from_string(&name);
            Ok(RhaiAxis {
                id: opt_id.ok_or_else(|| format!("no axis named {name:?}"))?,
                db: Arc::new(axis_system.clone()),
            })
        },
    );
}

#[derive(Debug, Clone)]
pub struct RhaiAxisSystem(pub RhaiTwistSystemBuilder);
impl LockAs<AxisSystemBuilder> for RhaiAxisSystem {
    fn lock(&self) -> Result<MappedMutexGuard<'_, AxisSystemBuilder>> {
        Ok(MappedMutexGuard::map(self.0.lock()?, |twists| {
            &mut twists.axes
        }))
    }
}
