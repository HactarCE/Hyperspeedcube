use super::Result;

/// Rhai native call context.
pub(crate) type Ctx<'a> = rhai::NativeCallContext<'a>;
/// Rhai evaluation context.
pub(crate) type EvalCtx<'a, 's, 'ps, 'g, 'c, 't> = rhai::EvalContext<'a, 's, 'ps, 'g, 'c, 't>;

/// Rhai context that can be used to call functions.
pub(crate) trait RhaiCtx {
    /// See [`Ctx::call_fn()`].
    fn call_rhai_fn<T: rhai::Variant + Clone>(
        &mut self,
        fn_name: impl AsRef<str>,
        args: impl rhai::FuncArgs,
    ) -> Result<T>;
    /// See [`Ctx::call_native_fn()`].
    fn call_rhai_native_fn<T: rhai::Variant + Clone>(
        &mut self,
        fn_name: impl AsRef<str>,
        args: impl rhai::FuncArgs,
    ) -> Result<T>;
}
impl RhaiCtx for &Ctx<'_> {
    fn call_rhai_fn<T: rhai::Variant + Clone>(
        &mut self,
        fn_name: impl AsRef<str>,
        args: impl rhai::FuncArgs,
    ) -> Result<T> {
        self.call_fn(fn_name, args)
    }

    fn call_rhai_native_fn<T: rhai::Variant + Clone>(
        &mut self,
        fn_name: impl AsRef<str>,
        args: impl rhai::FuncArgs,
    ) -> Result<T> {
        self.call_native_fn(fn_name, args)
    }
}
impl RhaiCtx for &mut EvalCtx<'_, '_, '_, '_, '_, '_> {
    fn call_rhai_fn<T: rhai::Variant + Clone>(
        &mut self,
        fn_name: impl AsRef<str>,
        args: impl rhai::FuncArgs,
    ) -> Result<T> {
        self.call_fn(fn_name, args)
    }

    fn call_rhai_native_fn<T: rhai::Variant + Clone>(
        &mut self,
        fn_name: impl AsRef<str>,
        args: impl rhai::FuncArgs,
    ) -> Result<T> {
        self.call_native_fn(fn_name, args)
    }
}
impl<C: RhaiCtx> RhaiCtx for &mut C {
    fn call_rhai_fn<T: rhai::Variant + Clone>(
        &mut self,
        fn_name: impl AsRef<str>,
        args: impl rhai::FuncArgs,
    ) -> Result<T> {
        (**self).call_rhai_fn(fn_name, args)
    }

    fn call_rhai_native_fn<T: rhai::Variant + Clone>(
        &mut self,
        fn_name: impl AsRef<str>,
        args: impl rhai::FuncArgs,
    ) -> Result<T> {
        (**self).call_rhai_native_fn(fn_name, args)
    }
}
