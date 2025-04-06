use parking_lot::{MappedMutexGuard, MutexGuard};

use super::*;

pub type RhaiColor = RhaiPuzzleElement<Color>;

pub fn init_engine(engine: &mut Engine) {
    engine.register_type_with_name::<RhaiColor>("color");
}

pub fn register(module: &mut Module) {
    new_fn("to_string").set_into_module(module, |color: &mut RhaiColor| {
        Some(color.lock_db().names.get(color.id)?.spec.clone())
    });
    new_fn("to_debug").set_into_module(module, |color: &mut RhaiColor| {
        format!("color({})", color.id)
    });

    new_fn("==").set_into_module(module, |c1: RhaiColor, c2: RhaiColor| c1 == c2);
    new_fn("!=").set_into_module(module, |c1: RhaiColor, c2: RhaiColor| c1 != c2);

    new_fn("set_name").set_into_module(
        module,
        |ctx: Ctx<'_>, color: &mut RhaiColor, name_spec| -> Result<()> {
            color
                .lock_db()
                .names
                .set(color.id, name_spec)
                .or_else(warnf(&ctx))
        },
    );

    FuncRegistration::new_getter("namespec").set_into_module(module, |color: &mut RhaiColor| {
        Some(color.lock_db().names.get(color.id)?.spec.clone())
    });
    FuncRegistration::new_getter("name").set_into_module(module, |color: &mut RhaiColor| {
        Some(color.lock_db().names.get(color.id)?.preferred.clone())
    });
}

impl RhaiColor {
    fn lock_db(&self) -> MappedMutexGuard<'_, ColorSystemBuilder> {
        MutexGuard::map(self.db.lock(), |puzzle| &mut puzzle.shape.colors)
    }
}
