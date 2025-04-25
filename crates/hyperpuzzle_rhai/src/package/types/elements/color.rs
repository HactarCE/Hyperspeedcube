use super::*;

pub fn register(module: &mut Module) {
    new_fn("set_name").set_into_module(
        module,
        |ctx: Ctx<'_>, color: &mut RhaiColor, name_spec| -> Result {
            color
                .lock_db()?
                .names
                .set(color.id, name_spec)
                .or_else(warnf(&ctx))
        },
    );

    FuncRegistration::new_getter("namespec").set_into_module(
        module,
        |color: &mut RhaiColor| -> Result<String> {
            Ok(color
                .lock_db()?
                .names
                .get(color.id)
                .ok_or("invalid color")?
                .spec
                .clone())
        },
    );
    FuncRegistration::new_getter("name").set_into_module(
        module,
        |color: &mut RhaiColor| -> Result<String> {
            Ok(color
                .lock_db()?
                .names
                .get(color.id)
                .ok_or("invalid color")?
                .preferred
                .clone())
        },
    );
}
