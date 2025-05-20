use std::{borrow::Cow, collections::HashMap, fmt, hash::Hash, sync::Arc};

use arcstr::Substr;
use ecow::EcoString;
use hypermath::Vector;
use itertools::Itertools;

use crate::{Error, ErrorMsg, EvalCtx, FileId, FnType, Result, Scope, Span, Type};

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

    pub fn type_error(&self, expected: Type) -> Error {
        ErrorMsg::TypeError {
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
        if self.data.ty().is_subtype_of(&expected) {
            Ok(())
        } else {
            Err(self.type_error(expected.into_owned()))
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
#[derive(Debug, Default, Clone)]
pub enum ValueData {
    #[default]
    Null,
    Bool(bool),
    Num(f64),
    Str(EcoString),
    List(Arc<Vec<Value>>),
    Map(Arc<HashMap<MapKey, Value>>),
    Fn(Arc<FnValue>),

    Vector(Vector),

    EuclidPoint(hypermath::Point),
    EuclidTransform(hypermath::pga::Motor),
    EuclidPlane(Box<hypermath::Hyperplane>),
    EuclidRegion(TODO),
}
impl fmt::Display for ValueData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Num(n) => match hypermath::to_approx_integer(*n) {
                Some(i) => write!(f, "{i}"),
                None => write!(f, "{n}"),
            },
            Self::Str(s) => write!(f, "{s}"),
            Self::List(values) => {
                write!(f, "[")?;
                let mut first = true;
                for v in &**values {
                    if !std::mem::take(&mut first) {
                        write!(f, ", ")?;
                    }
                    write!(f, "{v}")?;
                }
                write!(f, "]")?;
                Ok(())
            }
            Self::Map(hash_map) => {
                write!(f, "#{{")?;
                let mut first = true;
                for (k, v) in &**hash_map {
                    if !std::mem::take(&mut first) {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: {v}")?;
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
            Self::Vector(vector) => write!(f, "vec{vector}"),
            Self::EuclidPoint(point) => write!(f, "point{point}"),
            Self::EuclidTransform(motor) => todo!("display motor"),
            Self::EuclidPlane(hyperplane) => todo!("display hyperplane"),
            Self::EuclidRegion(todo) => todo!("display region"),
        }
    }
}
impl ValueData {
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
            Self::Vector(_) => Type::Vector,
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

    fn type_error(&self, expected: Type) -> ErrorMsg {
        ErrorMsg::TypeError {
            expected,
            got: self.ty(),
        }
    }

    pub fn to_num(&self, span: Span) -> Result<f64> {
        match self {
            Self::Num(n) => Ok(*n),
            _ => Err(self.type_error(Type::Num).at(span)),
        }
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
    pub fn get_overload(&self, span: Span, args_types: &[Type]) -> Result<&FnOverload> {
        let mut matching_dispatches = self
            .overloads
            .iter()
            .filter(|func| func.ty.might_take(args_types));
        let first_match = matching_dispatches.next().ok_or(
            ErrorMsg::BadArgTypes(self.overloads.iter().map(|f| f.ty.clone()).collect()).at(span),
        )?;
        let mut remaining = matching_dispatches.map(|func| &func.ty).collect_vec();
        if !remaining.is_empty() {
            remaining.insert(0, &first_match.ty);
            return Err(
                ErrorMsg::AmbiguousFnCall(remaining.into_iter().cloned().collect()).at(span),
            );
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
            return Err(ErrorMsg::FnOverloadConflict {
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
    pub fn call(&self, span: Span, ctx: &mut EvalCtx<'_>, args: Vec<Value>) -> Result<Value> {
        let overload = self.get_overload(span, &args.iter().map(|v| v.ty()).collect_vec())?;

        let scope = Scope::new_closure(Arc::clone(&ctx.scope), self.name.clone());
        // TODO: construct the new context within `call` so that we don't need
        //       to do it for builtins
        let mut call_ctx = EvalCtx {
            scope: &scope,
            runtime: ctx.runtime,
        };
        let return_value =
            (overload.call)(&mut call_ctx, args).or_else(Error::try_resolve_return_value)?;
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
        match self.debug_info {
            FnDebugInfo::Span(span) => span,
            FnDebugInfo::Internal(_) => crate::BUILTIN_SPAN,
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
