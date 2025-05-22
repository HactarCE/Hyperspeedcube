use std::borrow::{Borrow, Cow};
use std::fmt;
use std::hash::Hash;
use std::sync::Arc;

use arcstr::Substr;
use ecow::EcoString;
use hypermath::Vector;
use indexmap::IndexMap;
use itertools::Itertools;

use crate::{
    DiagMsg, EvalCtx, FnType, FullDiagnostic, Result, Scope, Span, Spanned, TracebackLine, Type,
};

/// Value in the language, with an optional associated span.
///
/// This type is relatively cheap to clone, especially for common types.
#[derive(Debug, Clone)]
pub struct Value {
    pub data: ValueData,
    pub span: Span,
}
impl std::ops::Deref for Value {
    type Target = ValueData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
impl std::ops::DerefMut for Value {
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
    pub const NULL: Self = Self {
        data: ValueData::Null,
        span: crate::BUILTIN_SPAN,
    };

    pub fn eq(&self, other: &Self, span: Span) -> Result<bool> {
        if std::mem::discriminant(&self.data) != std::mem::discriminant(&other.data) {
            return Ok(false);
        }

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

            _ => Err(DiagMsg::InvalidComparison(
                Box::new((self.ty(), self.span)),
                Box::new((other.ty(), other.span)),
            )
            .at(span)),
        }
    }

    pub fn repr(&self) -> String {
        format!("{:?}", self.data)
    }

    pub fn type_error(&self, expected: Type) -> FullDiagnostic {
        DiagMsg::TypeError {
            expected,
            got: self.ty(),
        }
        .at(self.span)
    }

    /// Check that a value has this type, and return an error if it doesn't.
    pub fn typecheck<'a>(&self, expected: impl Into<Cow<'a, Type>>) -> Result<()> {
        let expected = expected.into();
        if matches!(*expected, Type::Any) {
            return Ok(());
        }
        match (&self.data, &*expected) {
            (ValueData::List(list), Type::List(expected_inner_ty)) => {
                for elem in &**list {
                    elem.typecheck(&**expected_inner_ty)?;
                }
                Ok(())
            }
            (ValueData::Map(map), Type::Map(expected_inner_ty)) => {
                for elem in map.values() {
                    elem.typecheck(&**expected_inner_ty)?;
                }
                Ok(())
            }
            (ValueData::Fn(f), Type::Fn(fn_type)) if f.any_overload_is_subtype_of(fn_type) => {
                Ok(())
            }
            _ if self.data.ty().is_subtype_of(&expected) => Ok(()),
            _ => Err(self.type_error(expected.into_owned())),
        }
    }
    /// Returns whether the value has this type.
    pub fn is_type(&self, expected: &Type) -> bool {
        if matches!(expected, Type::Any) {
            return true;
        }
        match (&self.data, &*expected) {
            (ValueData::List(list), Type::List(expected_inner_ty)) => {
                list.iter().all(|elem| elem.is_type(expected_inner_ty))
            }
            (ValueData::Map(map), Type::Map(expected_inner_ty)) => {
                map.values().all(|value| value.is_type(expected_inner_ty))
            }
            (ValueData::Fn(f), Type::Fn(fn_type)) => f.any_overload_is_subtype_of(fn_type),
            _ => self.data.ty().is_subtype_of(&expected),
        }
    }

    pub fn as_func(&self) -> Result<&Arc<FnValue>> {
        match &self.data {
            ValueData::Fn(f) => Ok(f),
            _ => Err(self.type_error(Type::Fn(Box::new(FnType::default())))),
        }
    }
    /// Returns the function value. Replaces other values with a new function
    /// value.
    pub fn as_func_mut(&mut self, span: Span) -> &mut FnValue {
        if !matches!(self.data, ValueData::Fn(_)) {
            *self = ValueData::Fn(Arc::new(FnValue::default())).at(span);
        }
        match &mut self.data {
            ValueData::Fn(f) => Arc::make_mut(f),
            _ => unreachable!(),
        }
    }

    pub fn as_str(&self) -> Result<&EcoString> {
        match &self.data {
            ValueData::Str(s) => Ok(s),
            _ => Err(self.type_error(Type::Str)),
        }
    }

    pub(crate) fn as_bool(&self) -> Result<bool> {
        match &self.data {
            ValueData::Bool(b) => Ok(*b),
            _ => Err(self.type_error(Type::Bool)),
        }
    }

    pub(crate) fn as_num(&self) -> Result<f64> {
        match &self.data {
            ValueData::Num(n) => Ok(*n),
            _ => Err(self.type_error(Type::Num)),
        }
    }
    pub(crate) fn as_int(&self) -> Result<i64> {
        let n = self.as_num()?;
        hypermath::to_approx_integer(n).ok_or(DiagMsg::ExpectedInteger(n).at(self.span))
    }
    pub(crate) fn as_index(&self) -> Result<Index> {
        Ok(Index::from(self.as_int()?))
    }
    pub(crate) fn as_u8(&self) -> Result<u8> {
        let i = self.as_int()?;
        i.try_into().map_err(|_| {
            let bounds = Some((u8::MIN as i64, u8::MAX as i64));
            DiagMsg::IndexOutOfBounds { got: i, bounds }.at(self.span)
        })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Index {
    Front(usize),
    Back(usize),
}
impl From<i64> for Index {
    fn from(value: i64) -> Self {
        match value {
            0.. => Index::Front(value.try_into().unwrap_or(usize::MAX)),
            ..0 => Index::Back((-value - 1).try_into().unwrap_or(usize::MAX)),
        }
    }
}
impl Index {
    fn to_i64(self) -> i64 {
        match self {
            Index::Front(i) => i.try_into().unwrap_or(i64::MAX),
            Index::Back(i) => i
                .try_into()
                .unwrap_or(i64::MAX)
                .saturating_neg()
                .saturating_sub(1),
        }
    }
    fn to_usize(self) -> Option<usize> {
        match self {
            Index::Front(i) => i.try_into().ok(),
            Index::Back(_) => None,
        }
    }
    pub fn out_of_bounds_only_pos_err(self, len: usize) -> DiagMsg {
        DiagMsg::IndexOutOfBounds {
            got: self.to_i64(),
            bounds: len.checked_sub(1).and_then(|max| {
                let max: i64 = max.try_into().unwrap_or(i64::MAX);
                Some((0, max))
            }),
        }
    }
    pub fn out_of_bounds_pos_neg_err(self, len: usize) -> DiagMsg {
        DiagMsg::IndexOutOfBounds {
            got: self.to_i64(),
            bounds: len.checked_sub(1).and_then(|max| {
                let max: i64 = max.try_into().unwrap_or(i64::MAX);
                Some((max.saturating_neg().saturating_sub(1), max))
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub enum MapKey {
    Substr(Substr),
    String(EcoString),
}
impl AsRef<str> for MapKey {
    fn as_ref(&self) -> &str {
        match self {
            MapKey::Substr(s) => &s,
            MapKey::String(s) => &s,
        }
    }
}
impl Borrow<str> for MapKey {
    fn borrow(&self) -> &str {
        self.as_ref()
    }
}
impl PartialEq for MapKey {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref() == other.as_ref()
    }
}
impl Eq for MapKey {}
impl fmt::Display for MapKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_ref().fmt(f)
    }
}
impl Hash for MapKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state);
    }
}

/// Value in the language.
///
/// This type is relatively cheap to clone, especially for common types.
#[derive(Default, Clone)]
pub enum ValueData {
    #[default]
    Null,
    Bool(bool),
    Num(f64),
    Str(EcoString),
    List(Arc<Vec<Value>>),             // TODO: use rpds::Vector
    Map(Arc<IndexMap<MapKey, Value>>), // TODO: use rpds::RedBlackTreeMap
    Fn(Arc<FnValue>),                  // TODO: use rpds::Vector

    Vec(Vector),

    EuclidPoint(hypermath::Point),
    EuclidTransform(hypermath::pga::Motor),
    EuclidPlane(Box<hypermath::Hyperplane>),
    EuclidRegion(TODO),
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
                    v.fmt_internal(f, is_debug);
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
                    match k {
                        MapKey::Substr(k) => write!(f, "{k}")?,
                        MapKey::String(k) => write!(f, "{:?}", Self::Str(k.clone()))?,
                    }
                    write!(f, ": ")?;
                    v.fmt_internal(f, is_debug);
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
            Self::EuclidPoint(point) => {
                write!(f, "point(")?;
                fmt_comma_sep_numbers(&point.0.0, f, is_debug)?;
                write!(f, ")")?;
                Ok(())
            }
            Self::EuclidTransform(motor) => todo!("display motor"),
            Self::EuclidPlane(hyperplane) => todo!("display hyperplane"),
            Self::EuclidRegion(todo) => todo!("display region"),
        }
    }

    pub fn ty(&self) -> Type {
        match self {
            Self::Null => Type::Null,
            Self::Bool(_) => Type::Bool,
            Self::Num(_) => Type::Num,
            Self::Str(_) => Type::Str,
            Self::List(list) => Type::List(Box::new(list.iter().map(|v| v.ty()).collect())),
            Self::Map(map) => Type::Map(Box::new(map.values().map(|v| v.ty()).collect())),
            Self::Fn(func) => Type::Fn(Box::new(match func.overloads.as_slice() {
                [f] => f.ty.clone(),
                _ => FnType::default(),
            })),
            Self::Vec(_) => Type::Vec,
            Self::EuclidPoint(_) => Type::EuclidPoint,
            Self::EuclidTransform(_) => Type::EuclidTransform,
            Self::EuclidPlane(_) => Type::EuclidPlane,
            Self::EuclidRegion(_) => Type::EuclidRegion,
        }
    }

    pub fn at(self, span: Span) -> Value {
        Value { data: self, span }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
    pub fn is_map(&self) -> bool {
        matches!(self, Self::Map(_))
    }
}
impl From<()> for ValueData {
    fn from((): ()) -> Self {
        ValueData::Null
    }
}
impl From<bool> for ValueData {
    fn from(value: bool) -> Self {
        ValueData::Bool(value)
    }
}
impl From<f64> for ValueData {
    fn from(value: f64) -> Self {
        ValueData::Num(value)
    }
}
impl From<EcoString> for ValueData {
    fn from(value: EcoString) -> Self {
        ValueData::Str(value)
    }
}
impl From<&str> for ValueData {
    fn from(value: &str) -> Self {
        ValueData::Str(value.into())
    }
}
impl From<usize> for ValueData {
    fn from(value: usize) -> Self {
        ValueData::Num(value as f64)
    }
}
impl From<char> for ValueData {
    fn from(value: char) -> Self {
        ValueData::Str(value.into())
    }
}
impl FromIterator<Value> for ValueData {
    fn from_iter<T: IntoIterator<Item = Value>>(iter: T) -> Self {
        Self::List(Arc::new(iter.into_iter().collect()))
    }
}
impl<K: Into<MapKey>> FromIterator<(K, Value)> for ValueData {
    fn from_iter<T: IntoIterator<Item = (K, Value)>>(iter: T) -> Self {
        Self::Map(Arc::new(
            iter.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        ))
    }
}
impl From<Vec<Value>> for ValueData {
    fn from(value: Vec<Value>) -> Self {
        Self::List(Arc::new(value))
    }
}
impl From<Arc<Vec<Value>>> for ValueData {
    fn from(value: Arc<Vec<Value>>) -> Self {
        Self::List(value)
    }
}
impl From<Vector> for ValueData {
    fn from(value: Vector) -> Self {
        Self::Vec(value)
    }
}
impl From<hypermath::Point> for ValueData {
    fn from(value: hypermath::Point) -> Self {
        Self::EuclidPoint(value)
    }
}

// TODO: delete this
#[derive(Debug, Clone)]
struct TODO;

#[derive(Debug, Default, Clone)]
pub struct FnValue {
    pub name: Option<Substr>,
    pub overloads: Vec<FnOverload>,
}
impl FnValue {
    pub fn guess_return_type(&self, arg_types: &[Type]) -> Option<Type> {
        self.overloads
            .iter()
            .filter(|func| func.ty.might_take(arg_types))
            .map(|func| func.ty.ret.clone())
            .reduce(Type::unify)
    }
    /// Returns whether this function can be called as a method for a value of
    /// type `ty`.
    pub fn can_be_method_of(&self, ty: Type) -> bool {
        !matches!(ty, Type::Map(_))
            && self.overloads.iter().any(|func| {
                func.ty.params.as_ref().is_none_or(|param_types| {
                    param_types
                        .first()
                        .is_some_and(|param_type| ty.overlaps(param_type))
                })
            })
    }
    pub fn any_overload_is_subtype_of(&self, fn_type: &FnType) -> bool {
        self.overloads
            .iter()
            .any(|overload| overload.ty.is_subtype_of(fn_type))
    }
    pub fn get_overload(&self, fn_span: Span, args: &[Value]) -> Result<&FnOverload> {
        let mut matching_dispatches = self
            .overloads
            .iter()
            .filter(|func| func.ty.would_take(args));
        let first_match = matching_dispatches.next().ok_or_else(|| {
            DiagMsg::BadArgTypes {
                arg_types: args.iter().map(|arg| (arg.ty(), arg.span)).collect(),
                overloads: self.overloads.iter().map(|f| f.ty.clone()).collect(),
            }
            .at(fn_span)
        })?;
        let mut remaining = matching_dispatches.map(|func| &func.ty).collect_vec();
        if !remaining.is_empty() {
            remaining.insert(0, &first_match.ty);
            return Err(DiagMsg::AmbiguousFnCall {
                arg_types: args.iter().map(|arg| (arg.ty(), arg.span)).collect(),
                overloads: remaining.into_iter().cloned().collect(),
            }
            .at(fn_span));
        }
        Ok(first_match)
    }
    pub fn push_overload(&mut self, overload: FnOverload) -> Result<()> {
        #[cfg(debug_assertions)]
        if let Some(conflict) = self
            .overloads
            .iter()
            .find(|existing| existing.ty.might_conflict_with(&overload.ty))
        {
            return Err(DiagMsg::FnOverloadConflict {
                new_ty: Box::new(overload.ty),
                old_ty: Box::new(conflict.ty.clone()),
                old_span: match conflict.debug_info {
                    FnDebugInfo::Span(span) => Some(span),
                    FnDebugInfo::Internal(_) => None,
                },
            }
            .debug_at(overload.debug_info));
        }

        self.overloads.push(overload);

        Ok(())
    }
    pub fn call(
        &self,
        call_span: Span,
        fn_span: Span,
        ctx: &mut EvalCtx<'_>,
        args: Vec<Value>,
    ) -> Result<Value> {
        let overload = self.get_overload(fn_span, &args)?;

        let scope = Scope::new_closure(Arc::clone(&ctx.scope), self.name.clone());
        // TODO: construct the new context within `call` so that we don't need
        //       to do it for builtins
        let mut call_ctx = EvalCtx {
            scope: &scope,
            runtime: ctx.runtime,
            caller_span: call_span,
        };
        let return_value = (overload.call)(&mut call_ctx, args)
            .or_else(FullDiagnostic::try_resolve_return_value)
            .map_err(|e| {
                e.at_caller(TracebackLine {
                    fn_name: self.name.clone(),
                    fn_span: overload.opt_span(),
                    call_span,
                })
            })?;
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

#[derive(Clone)]
pub struct FnOverload {
    pub ty: FnType,
    pub call: Arc<dyn Send + Sync + Fn(&mut EvalCtx<'_>, Vec<Value>) -> Result<Value>>,
    pub debug_info: FnDebugInfo,
}
impl fmt::Debug for FnOverload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FunctionOverload")
            .field("ty", &self.ty)
            .finish()
    }
}
impl FnOverload {
    pub fn span(&self) -> Span {
        self.opt_span().unwrap_or(crate::BUILTIN_SPAN)
    }
    fn opt_span(&self) -> Option<Span> {
        match self.debug_info {
            FnDebugInfo::Span(span) => Some(span),
            FnDebugInfo::Internal(_) => None,
        }
    }
    pub fn internal_name(&self) -> Option<&'static str> {
        match self.debug_info {
            FnDebugInfo::Span(_) => None,
            FnDebugInfo::Internal(name) => Some(name),
        }
    }
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FnDebugInfo {
    Span(Span),
    Internal(&'static str),
}
impl From<Span> for FnDebugInfo {
    fn from(value: Span) -> Self {
        Self::Span(value)
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
