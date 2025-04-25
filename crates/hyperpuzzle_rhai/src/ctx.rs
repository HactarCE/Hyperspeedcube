use itertools::Itertools;
use rhai::Dynamic;

use super::Result;

/// Rhai native call context.
pub(crate) type Ctx<'a> = rhai::NativeCallContext<'a>;
/// Rhai evaluation context.
pub(crate) type EvalCtx<'a, 's, 'ps, 'g, 'c, 't> = rhai::EvalContext<'a, 's, 'ps, 'g, 'c, 't>;

/// Rhai context that can be used to call functions.
pub(crate) trait RhaiCtx {
    /// See [`Ctx::call_native_fn()`].
    fn call_rhai_native_fn<T: rhai::Variant + Clone>(
        &mut self,
        fn_name: &str,
        args: Vec<Dynamic>,
    ) -> Result<T>;

    /// Returns a nicer name for a type, given a name produced by
    /// [`std::any::type_name()`].
    fn map_type_name<'a>(&'a self, name: &'a str) -> &'a str;
}
impl RhaiCtx for &Ctx<'_> {
    fn call_rhai_native_fn<T: rhai::Variant + Clone>(
        &mut self,
        fn_name: &str,
        args: Vec<Dynamic>,
    ) -> Result<T> {
        self.call_native_fn(fn_name, args)
    }

    fn map_type_name<'a>(&'a self, name: &'a str) -> &'a str {
        self.engine().map_type_name(name)
    }
}
impl RhaiCtx for &mut EvalCtx<'_, '_, '_, '_, '_, '_> {
    fn call_rhai_native_fn<T: rhai::Variant + Clone>(
        &mut self,
        fn_name: &str,
        mut args: Vec<Dynamic>,
    ) -> Result<T> {
        // `false` so that we don't pass in a `this` pointer
        self.call_native_fn_raw(fn_name, false, &mut args.iter_mut().collect_vec())?
            .try_cast()
            .ok_or_else(|| format!("bad return type for {fn_name}").into())
    }

    fn map_type_name<'a>(&'a self, name: &'a str) -> &'a str {
        self.engine().map_type_name(name)
    }
}
impl<C: RhaiCtx> RhaiCtx for &mut C {
    fn call_rhai_native_fn<T: rhai::Variant + Clone>(
        &mut self,
        fn_name: &str,
        args: Vec<Dynamic>,
    ) -> Result<T> {
        (**self).call_rhai_native_fn(fn_name, args)
    }

    fn map_type_name<'a>(&'a self, name: &'a str) -> &'a str {
        (**self).map_type_name(name)
    }
}
