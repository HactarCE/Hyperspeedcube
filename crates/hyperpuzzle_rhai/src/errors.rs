use std::fmt;

use rhai::{Dynamic, EvalAltResult};

use crate::{FromRhai, Result, RhaiCtx};

/// Extension trait for [`eyre::Result<T>`] that converts eyre reports into Rhai
/// errors in a way that includes the error chain.
pub(crate) trait EyreRhai {
    type Output;

    /// Converts an [`eyre::Result`] into a Rhai [`Result`].
    fn eyrefmt(self) -> Result<Self::Output>;
}
impl<T> EyreRhai for eyre::Result<T> {
    type Output = T;

    fn eyrefmt(self) -> Result<T> {
        // use `{e:#}` to show all eyre context
        self.map_err(|e| format!("{e:#}").into())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ConvertError {
    pub expected: String,
    pub got: Option<String>,
    pub keys: Vec<String>,
}
impl fmt::Display for ConvertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            expected,
            got,
            keys: context,
        } = self;
        write!(f, "expected {expected}")?;
        let mut keys_iter = context.iter();
        if let Some(key) = keys_iter.next() {
            write!(f, " for {key}")?;
        }
        for key in keys_iter {
            write!(f, " in {key}")?;
        }
        write!(f, "; got {}", got.as_deref().unwrap_or("nothing"))?;
        Ok(())
    }
}
impl std::error::Error for ConvertError {}
impl From<ConvertError> for Box<EvalAltResult> {
    fn from(value: ConvertError) -> Self {
        value.to_string().into()
    }
}
impl ConvertError {
    /// Constructs a new conversion error, where `T` is the expected type and
    /// `got` is the value that was actually gotten.
    pub fn new<T: FromRhai>(ctx: impl RhaiCtx, got: Option<&Dynamic>) -> Self {
        Self::new_expected_str(ctx, T::expected_string(), got)
    }

    /// Constructs a new conversion error, where `expected` is a string
    /// representing the expected type and `got` is the value that was actually
    /// gotten.
    pub fn new_expected_str(
        mut ctx: impl RhaiCtx,
        expected: impl ToString,
        got: Option<&Dynamic>,
    ) -> Self {
        Self {
            expected: expected.to_string(),
            got: got.map(|v| {
                let debug_str = crate::util::rhai_to_debug(&mut ctx, v);
                let type_name = ctx.map_type_name(v.type_name());
                format!("{type_name} {debug_str}")
            }),
            keys: vec![],
        }
    }
}

/// Extension trait for `ConvertError` and `Result<T, ConvertError>`.
pub trait InKey: Sized {
    /// Adds context `format!("in {s}")` to a conversion error.
    fn in_structure(self, s: impl ToString) -> Self;
    /// Adds context `format!("in '{s}'")` (note the single quotes) to a
    /// conversion error.
    fn in_key(self, s: impl fmt::Display) -> Self {
        self.in_structure(format!("key '{s}'"))
    }
}
impl InKey for ConvertError {
    fn in_structure(mut self, s: impl ToString) -> Self {
        self.keys.push(s.to_string());
        self
    }
}
impl<T> InKey for Result<T, ConvertError> {
    fn in_structure(self, s: impl ToString) -> Self {
        self.map_err(|e| e.in_structure(s))
    }
}
