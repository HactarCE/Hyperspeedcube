use std::borrow::Cow;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use arcstr::Substr;
use ecow::EcoString;
use hypermath::{Vector, VectorRef};
use itertools::Itertools;

use crate::{
    BoxDynValue, Error, EvalCtx, FnType, FullDiagnostic, List, Map, ND_EUCLID, Result, Scope, Span,
    TracebackLine, Type,
};

/// Value in the language, with an optional associated span.
///
/// This type is relatively cheap to clone, especially for common types.
///
/// This type dereferences to [`ValueData`], so you can call [`ValueData`]
/// methods on it without a need for `.data`.
#[derive(Debug, Clone)]
pub struct Value {
    /// Data in the value.
    pub data: ValueData,
    /// Span where the value was constructed.
    pub span: Span,
}
impl Deref for Value {
    type Target = ValueData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
impl DerefMut for Value {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.fmt(f)
    }
}
impl Default for Value {
    fn default() -> Self {
        Self::NULL
    }
}
impl Value {
    /// `null` with no span.
    pub const NULL: Self = Self {
        data: ValueData::Null,
        span: crate::BUILTIN_SPAN,
    };

    /// Returns whether `self` and `other` are equal, or returns an error if
    /// they are both of a type that cannot be compared.
    pub fn eq(&self, other: &Self, span: Span) -> Result<bool> {
        if std::mem::discriminant(&self.data) != std::mem::discriminant(&other.data) {
            return Ok(false);
        }

        let invalid_comparison_error = || {
            Error::InvalidComparison(
                Box::new((self.ty(), self.span)),
                Box::new((other.ty(), other.span)),
            )
            .at(span)
        };

        match (&self.data, &other.data) {
            (ValueData::Null, ValueData::Null) => Ok(true),
            (ValueData::Bool(b1), ValueData::Bool(b2)) => Ok(b1 == b2),
            (ValueData::Num(n1), ValueData::Num(n2)) => Ok(hypermath::approx_eq(n1, n2)),
            (ValueData::Str(s1), ValueData::Str(s2)) => Ok(s1 == s2),
            (ValueData::List(l1), ValueData::List(l2)) => Ok(l1.len() == l2.len()
                && std::iter::zip(&**l1, &**l2)
                    .map(|(a, b)| Self::eq(a, b, span))
                    .fold_ok(true, |a, b| a && b)?),
            (ValueData::Map(m1), ValueData::Map(m2)) => Ok(m1.len() == m2.len()
                && m1
                    .iter()
                    .map(|(k, v1)| match m2.get(k) {
                        Some(v2) => Self::eq(v1, v2, span),
                        None => Ok(false),
                    })
                    .fold_ok(true, |v1, v2| v1 && v2)?),
            (ValueData::Vec(v1), ValueData::Vec(v2)) => Ok(hypermath::approx_eq(v1, v2)),
            (ValueData::EuclidPoint(point1), ValueData::EuclidPoint(point2)) => {
                Ok(hypermath::approx_eq(point1, point2))
            }
            (ValueData::EuclidTransform(motor1), ValueData::EuclidTransform(motor2)) => {
                Ok(Option::zip(motor1.canonicalize(), motor2.canonicalize())
                    .is_some_and(|(m1, m2)| hypermath::approx_eq(&m1, &m2)))
            }
            (ValueData::EuclidPlane(plane1), ValueData::EuclidPlane(plane2)) => {
                Ok(hypermath::approx_eq(&**plane1, &**plane2))
            }
            (ValueData::Custom(c1), ValueData::Custom(c2)) => {
                c1.eq(c2).ok_or_else(invalid_comparison_error)
            }

            _ => Err(invalid_comparison_error()),
        }
    }

    /// Returns a type error saying that this value has the wrong type.
    pub fn type_error(&self, expected: Type) -> FullDiagnostic {
        Error::TypeError {
            expected,
            got: self.ty(),
        }
        .at(self.span)
    }

    /// Checks the type for invalid floating-point values.
    ///
    /// This only performs a shallow check, so lists and maps are not checked.
    /// It's called after internal functions, which are the only ones that can
    /// actually generate new primitive values.
    pub(crate) fn check_for_invalid_floats(&self) -> Result<()> {
        match &self.data {
            ValueData::Num(n) => ensure_non_nan_number(*n, self.span),
            ValueData::Vec(v) | ValueData::EuclidPoint(hypermath::Point(v)) => {
                ensure_finite_numbers(v.iter(), self.span)
            }
            ValueData::EuclidTransform(t) => ensure_finite_numbers(t.coefs(), self.span),
            ValueData::EuclidPlane(p) => {
                ensure_finite_number(p.distance(), self.span)?;
                ensure_finite_numbers(p.normal().iter(), self.span)
            }
            _ => Ok(()),
        }
    }

    /// Check that a value has this type, and return an error if it doesn't.
    pub fn typecheck<'a>(&self, expected: impl Into<Cow<'a, Type>>) -> Result<()> {
        let expected: Cow<'a, Type> = expected.into();

        if let Type::List(Some(inner)) | Type::NonEmptyList(Some(inner)) = &*expected {
            if let Ok(l) = self.as_ref::<List>() {
                for elem in &**l {
                    elem.typecheck(&**inner)?;
                }
            }
        }

        let is_same_type = match &*expected {
            Type::Any => true,

            Type::List(_) => matches!(self.data, ValueData::List(_)),
            Type::EmptyList => matches!(&self.data, ValueData::List(l) if l.is_empty()),
            Type::NonEmptyList(_) => matches!(&self.data, ValueData::List(l) if !l.is_empty()),

            Type::Int => {
                self.ref_to::<i64>()?;
                true
            }
            Type::Nat => {
                self.ref_to::<u64>()?;
                true
            }

            Type::Union(types) => types.to_vec().iter().any(|ty| self.is_type(ty)),

            Type::Custom(s) => {
                matches!(&self.data, ValueData::Custom(v) if v.type_name() == *s)
            }

            _ => self.ty().is_subtype_of(&expected),
        };

        if is_same_type {
            Ok(())
        } else {
            Err(self.type_error(expected.into_owned()))
        }
    }
    /// Returns whether the value has this type.
    pub fn is_type(&self, expected: &Type) -> bool {
        // This could be faster by not constructing an error message.
        self.typecheck(expected).is_ok()
    }
}

/// Value in the language.
///
/// This type is relatively cheap to clone, especially for common types.
#[derive(Default, Clone)]
pub enum ValueData {
    /// Null.
    ///
    /// Used for optional values (analogous to [`None`]) and values with no data
    /// (analogous to the unit type `()`).
    #[default]
    Null,
    /// True or false.
    Bool(bool),
    /// Floating-point number. Also used in place of integers.
    Num(f64),
    /// Copy-on-write Unicode string.
    Str(EcoString),
    /// Copy-on-write list of values.
    List(Arc<List>), // TODO: use rpds::Vector
    /// Copy-on-write dictionary with string keys.
    Map(Arc<Map>), // TODO: use rpds::RedBlackTreeMap
    /// Function with a copy-on-write list of overloads.
    Fn(Arc<FnValue>), // TODO: use rpds::Vector

    /// N-dimensional vector of numbers. Unused dimensions are zero.
    Vec(Vector),

    /// Type.
    Type(Type),

    /// Point in Euclidean space.
    EuclidPoint(hypermath::Point),
    /// Isometry (distance-preserving transform) in Euclidean space.
    ///
    /// All isometries can be constructing by composing a translation, a
    /// rotation, and an optional reflection.
    EuclidTransform(hypermath::pga::Motor),
    /// Hyperplane in Euclidean space.
    ///
    /// In an N-dimensional space, this represents an (N-1)-dimensional plane.
    EuclidPlane(Box<hypermath::Hyperplane>),
    /// Region of Euclidean space.
    EuclidRegion(std::convert::Infallible),
    /// PGA blade in Euclidean space.
    EuclidBlade(hypermath::pga::Blade),

    /// Custom type defined in a downstream crate.
    Custom(BoxDynValue),
}
impl fmt::Debug for ValueData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_internal(f, true)
    }
}
impl fmt::Display for ValueData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_internal(f, false)
    }
}
impl ValueData {
    fn fmt_internal(&self, f: &mut fmt::Formatter<'_>, is_debug: bool) -> fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Num(n) if is_debug => write!(f, "{n}"),
            Self::Num(n) => match hypermath::to_approx_integer(*n) {
                Some(i) => write!(f, "{i}"),
                None => write!(f, "{n}"),
            },
            Self::Str(s) if is_debug => {
                write!(f, "\"")?;
                for c in s.chars() {
                    if crate::parse::CHARS_THAT_MUST_BE_ESCAPED_IN_STRING_LITERALS.contains(c) {
                        write!(f, "\\{c}")?;
                    } else {
                        write!(f, "{}", c.escape_debug())?;
                    }
                }
                write!(f, "\"")?;
                Ok(())
            }
            Self::Str(s) => write!(f, "{s}"),
            Self::List(list) => {
                write!(f, "[")?;
                let mut first = true;
                for v in &**list {
                    if !std::mem::take(&mut first) {
                        write!(f, ", ")?;
                    }
                    v.fmt_internal(f, is_debug)?;
                }
                write!(f, "]")?;
                Ok(())
            }
            Self::Map(map) => {
                write!(f, "#{{")?;
                let mut first = true;
                for (k, v) in &**map {
                    if !std::mem::take(&mut first) {
                        write!(f, ", ")?;
                    }
                    if crate::parse::is_valid_ident(k) {
                        write!(f, "{k}")?;
                    } else {
                        write!(f, "{k:?}")?;
                    }
                    write!(f, ": ")?;
                    v.fmt_internal(f, is_debug)?;
                }
                write!(f, "}}")?;
                Ok(())
            }
            Self::Fn(fn_value) => {
                if fn_value.overloads.len() == 1 {
                    write!(f, "{}", fn_value.overloads[0].ty)
                } else {
                    write!(f, "fn with {} overloads", fn_value.overloads.len())
                }
            }
            Self::Vec(vec) => {
                write!(f, "vec(")?;
                fmt_comma_sep_numbers(&vec.0, f, is_debug)?;
                write!(f, ")")?;
                Ok(())
            }
            Self::Type(ty) => write!(f, "{ty}"),
            Self::EuclidPoint(point) => {
                write!(f, "{ND_EUCLID}.point(")?;
                fmt_comma_sep_numbers(&point.0.0, f, is_debug)?;
                write!(f, ")")?;
                Ok(())
            }
            Self::EuclidTransform(motor) => write!(f, "{ND_EUCLID}.motor({motor})"),
            Self::EuclidPlane(hyperplane) => write!(f, "{ND_EUCLID}.plane({hyperplane})"),
            Self::EuclidRegion(region) => todo!("display region"),
            Self::EuclidBlade(blade) => write!(f, "{ND_EUCLID}.blade({blade})"),
            Self::Custom(value) => value.fmt(f, is_debug),
        }
    }

    /// Returns a debug representation of the value that is useful to the user.
    pub fn repr(&self) -> String {
        format!("{self:?}")
    }

    /// Returns the type of the value.
    pub fn ty(&self) -> Type {
        match self {
            Self::Null => Type::Null,
            Self::Bool(_) => Type::Bool,
            Self::Num(_) => Type::Num,
            Self::Str(_) => Type::Str,
            Self::List(list) => match list.iter().map(|v| v.ty()).reduce(Type::unify) {
                Some(unified_type) => Type::NonEmptyList(Some(Box::new(unified_type))),
                None => Type::EmptyList,
            },
            Self::Map(_) => Type::Map,
            Self::Fn(_) => Type::Fn,
            Self::Vec(_) => Type::Vec,
            Self::Type(_) => Type::Type,
            Self::EuclidPoint(_) => Type::EuclidPoint,
            Self::EuclidTransform(_) => Type::EuclidTransform,
            Self::EuclidPlane(_) => Type::EuclidPlane,
            Self::EuclidRegion(_) => Type::EuclidRegion,
            Self::EuclidBlade(_) => Type::EuclidBlade,
            Self::Custom(value) => Type::Custom(value.type_name()),
        }
    }

    /// Attaches a span to the value.
    pub fn at(self, span: Span) -> Value {
        Value { data: self, span }
    }

    /// Returns whether the value is `null`.
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
    /// Returns whether the value is a function.
    pub fn is_func(&self) -> bool {
        matches!(self, Self::Fn(_))
    }
}

/// Script function.
#[derive(Debug, Clone)]
pub struct FnValue {
    /// Name of the function, or `None` if it is anonymous.
    pub name: Option<Substr>,
    /// List of overloads, which may overlap although they typically don't.
    pub overloads: Vec<FnOverload>,
}
impl FnValue {
    /// Constructs a new function value with no overloads.
    pub fn new(name: Option<Substr>) -> Self {
        let overloads = vec![];
        Self { name, overloads }
    }

    /// Guesses the return type, given a list of arguments. Returns `None` if
    /// there is no overload matching the argument type.
    pub fn guess_return_type(&self, arg_types: &[Type]) -> Option<Type> {
        self.overloads
            .iter()
            .filter(|func| func.ty.might_take(arg_types))
            .map(|func| func.ty.ret.clone())
            .reduce(Type::unify)
    }
    /// Returns whether this function can be called as a method for a value of
    /// type `ty`.
    pub fn can_be_method_of(&self, ty: &Type) -> bool {
        !matches!(ty, Type::Map)
            && self
                .overloads
                .iter()
                .any(|func| match func.ty.params.first() {
                    None => func.ty.is_variadic,
                    Some(first_param_ty) => ty.is_subtype_of(first_param_ty),
                })
    }
    /// Returns whether any overload of the function is a subtype of `fn_type`.
    pub fn any_overload_is_subtype_of(&self, fn_type: &FnType) -> bool {
        self.overloads
            .iter()
            .any(|overload| overload.ty.is_subtype_of(fn_type))
    }
    /// Returns the overload to use when calling the function with `args`, or an
    /// error if there is no matching overload or multiple matching overloads.
    pub fn get_overload(&self, fn_span: Span, args: &[Value]) -> Result<&FnOverload> {
        let mut matching_dispatches = self
            .overloads
            .iter()
            .filter(|func| func.ty.would_take(args));
        let first_match = matching_dispatches.next().ok_or_else(|| {
            Error::BadArgTypes {
                arg_types: args.iter().map(|arg| (arg.ty(), arg.span)).collect(),
                overloads: self.overloads.iter().map(|f| f.ty.clone()).collect(),
            }
            .at(fn_span)
        })?;
        let mut remaining = matching_dispatches.map(|func| &func.ty).collect_vec();
        if !remaining.is_empty() {
            remaining.insert(0, &first_match.ty);
            return Err(Error::AmbiguousFnCall {
                arg_types: args.iter().map(|arg| (arg.ty(), arg.span)).collect(),
                overloads: remaining.into_iter().cloned().collect(),
            }
            .at(fn_span));
        }
        Ok(first_match)
    }
    /// Adds an overload to the function. Returns an error if the new overload
    /// overlaps with an existing one.
    pub fn push_overload(&mut self, overload: FnOverload) -> Result<()> {
        if crate::CHECK_FN_OVERLOAD_CONFLICTS {
            if let Some(conflict) = self
                .overloads
                .iter()
                .find(|existing| existing.ty.might_conflict_with(&overload.ty))
            {
                let error = Error::FnOverloadConflict {
                    new_ty: Box::new(overload.ty),
                    old_ty: Box::new(conflict.ty.clone()),
                    old_span: match conflict.debug_info {
                        FnDebugInfo::Span(span) => Some(span),
                        FnDebugInfo::Internal(_) => None,
                    },
                }
                .at(overload.debug_info.to_span().unwrap_or(crate::BUILTIN_SPAN));

                #[cfg(debug_assertions)]
                if let FnDebugInfo::Internal(name) = overload.debug_info {
                    panic!("error in internal {name:?}: {error:?}")
                }

                return Err(error);
            }
        }

        self.overloads.push(overload);

        Ok(())
    }
    /// Calls the function, using the built-in span for the call site.
    pub fn call(
        &self,
        fn_span: Span,
        ctx: &mut EvalCtx<'_>,
        args: List,
        kwargs: Map,
    ) -> Result<Value> {
        self.call_at(crate::BUILTIN_SPAN, fn_span, ctx, args, kwargs)
    }
    /// Calls the function.
    ///
    /// - `call_span` should be a span containing the function being called and
    ///   its arguments.
    /// - `fn_span` should be a span containing one of:
    ///     - the function value where it is called
    ///     - the function's definition
    pub fn call_at(
        &self,
        call_span: Span,
        fn_span: Span,
        ctx: &mut EvalCtx<'_>,
        args: List,
        kwargs: Map,
    ) -> Result<Value> {
        let overload = self.get_overload(fn_span, &args)?;

        let fn_scope = match &overload.parent_scope {
            Some(parent) => Cow::Owned(Scope::new_closure(
                ctx.scope,
                Arc::clone(parent),
                self.name.clone(),
            )),
            None => Cow::Borrowed(ctx.scope),
        };
        let mut exports = None;
        let mut call_ctx = EvalCtx {
            scope: &fn_scope,
            runtime: ctx.runtime,
            caller_span: call_span,
            exports: &mut exports,
        };
        let mut return_value = (overload.call)(&mut call_ctx, args, kwargs)
            .or_else(FullDiagnostic::try_resolve_return_value)
            .and_then(|return_value| {
                if matches!(overload.debug_info, FnDebugInfo::Internal(_)) {
                    return_value.check_for_invalid_floats()?;
                }
                Ok(return_value)
            })
            .map_err(|e| {
                e.at_caller(TracebackLine {
                    fn_name: self.name.clone(),
                    fn_span: overload.debug_info.to_span(),
                    call_span,
                })
            })?;
        if let Some(exports) = call_ctx.exports.take() {
            return_value = ValueData::Map(Arc::new(exports)).at(call_span);
        }
        return_value.typecheck(&overload.ty.ret).map_err(|e| {
            if let FnDebugInfo::Internal(name) = overload.debug_info {
                if cfg!(debug_assertions) {
                    panic!("bad return type for built-in function {name:?}: {e:?}");
                }
            }
            e
        })?;
        Ok(return_value)
    }
}

/// Overload for a function.
#[derive(Clone)]
pub struct FnOverload {
    /// Function type signature.
    pub ty: FnType,
    /// Function to evaluate the function body.
    pub call: Arc<dyn Send + Sync + Fn(&mut EvalCtx<'_>, List, Map) -> Result<Value>>,
    /// Debug info about the source of the function.
    pub debug_info: FnDebugInfo,
    /// Parent scope to use for the function. If this is `None`, then the
    /// function uses the caller's scope, which is an optimization that is only
    /// valid for for built-in functions.
    pub parent_scope: Option<Arc<Scope>>,
    /// Documentation lines.
    pub docs: Option<&'static [&'static str]>,
}
impl fmt::Debug for FnOverload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FunctionOverload")
            .field("ty", &self.ty)
            .finish()
    }
}

/// Debug info about the source of a function.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FnDebugInfo {
    /// Span where the function was defined in user code.
    Span(Span),
    /// Internal name of the function.
    Internal(&'static str),
}
impl From<Span> for FnDebugInfo {
    fn from(value: Span) -> Self {
        Self::Span(value)
    }
}
impl FnDebugInfo {
    /// Returns the span where the overload was defined, or `None` if the
    /// overload is built-in.
    fn to_span(self) -> Option<Span> {
        match self {
            FnDebugInfo::Span(span) => Some(span),
            FnDebugInfo::Internal(_) => None,
        }
    }
}

fn fmt_comma_sep_numbers(
    numbers: &[f64],
    f: &mut fmt::Formatter<'_>,
    is_debug: bool,
) -> fmt::Result {
    let mut is_first = true;
    for &n in numbers {
        if !std::mem::take(&mut is_first) {
            write!(f, ", ")?;
        }
        ValueData::Num(n).fmt_internal(f, is_debug)?;
    }
    Ok(())
}

#[inline(always)]
fn ensure_non_nan_number(n: f64, span: Span) -> Result<()> {
    if n.is_nan() {
        Err(Error::NaN.at(span))
    } else {
        Ok(())
    }
}
#[inline(always)]
fn ensure_finite_number(n: f64, span: Span) -> Result<()> {
    ensure_non_nan_number(n, span)?;
    if n.is_infinite() {
        Err(Error::Infinity.at(span))
    } else {
        Ok(())
    }
}
#[inline(always)]
fn ensure_finite_numbers(ns: impl Iterator<Item = f64>, span: Span) -> Result<()> {
    for n in ns {
        ensure_finite_number(n, span)?;
    }
    Ok(())
}
