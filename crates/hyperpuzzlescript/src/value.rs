use std::{collections::HashMap, fmt, sync::Arc};

use arcstr::Substr;
use ecow::EcoString;
use hypermath::{Vector, approx_eq};
use itertools::Itertools;
use parking_lot::Mutex;

use crate::{ErrorMsg, Result, Span, Spanned, Type, eval::Ctx, ty::FnType};

/// Value in the language, with an optional associated span.
///
/// This type is relatively cheap to clone, especially for common types.
#[derive(Debug, Default, Clone)]
pub struct Value {
    pub data: ValueData,
    pub span: Option<Span>,
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
        write!(f, "")
    }
}
impl Value {
    pub fn type_error(&self, expected: Type) -> ErrorMsg
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
    Map(Arc<HashMap<Substr, Value>>),
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
            Self::Num(n) => {
                if approx_eq(n, &n.round()) {
                    write!(f, "{}", (n.round() as i64))
                } else {
                    write!(f, "{}", n)
                }
            }
            Self::Str(s) => write!(f, "{s}"),
            Self::List(values) => todo!("display recursively"),
            Self::Map(hash_map) => todo!("display recursively"),
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
            Value::Num(n) => Ok(*n),
            _ => Err(self.type_error(Type::Num).at(span)),
        }
    }

    pub fn new_fn() -> Value {
        Value::Fn(Arc::new(FnValue::default()))
    }
    pub fn as_func(&self, span: Span) -> Result<&Arc<FnValue>> {
        match self {
            Value::Fn(f) => Ok(f),
            _ => Err(self.type_error(Type::Num).at(span)),
        }
    }
    /// Returns the function value. Replaces other values with a new function
    /// value.
    pub fn as_func_mut(&mut self) -> &mut FnValue {
        match self {
            Value::Fn(f) => Arc::make_mut(f),
            _ => {
                *self = Self::new_fn();
                self.as_func_mut()
            }
        }
    }

    pub fn as_str(&self, span: Span) -> Result<&EcoString> {
        match self {
            Value::Str(s) => Ok(s),
            _ => Err(self.type_error(Type::Num).at(span)),
        }
    }
}

// TODO: delete this
#[derive(Debug, Clone)]
struct TODO;

#[derive(Debug, Default, Clone)]
pub struct FnValue {
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
    pub fn get_overload(&self, args_types: &[Type]) -> Result<&FnOverload, FnCallError<'_>> {
        let mut matching_dispatches = self
            .overloads
            .iter()
            .filter(|func| func.ty.might_take(args_types));
        let first_match = matching_dispatches.next().ok_or(FnCallError::BadArgTypes)?;
        let mut remaining = matching_dispatches.map(|func| &func.ty).collect_vec();
        if !remaining.is_empty() {
            remaining.insert(0, &first_match.ty);
            return Err(FnCallError::Ambiguous(remaining));
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
            return Err(ErrorMsg::CannotModifyInOuterScope);
            panic!("conflict between {conflict:?} and {overload:?}")
        }

        self.overloads.push(value);
    }
    pub fn call(&self, ctx: &mut Ctx, args: &[Value]) -> Result<Value, String> {
        let overload = self
            .get_overload(&args.iter().map(|v| v.ty()).collect_vec())
            .map_err(|e| format!("{e:?}"))?;
        (overload.ptr)(ctx, args)
    }
}

pub struct FnOverload {
    pub ty: FnType,
    pub ptr: Box<dyn Fn(&mut Ctx, &[Value]) -> Result<Value, String>>,
}
impl fmt::Debug for FnOverload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FunctionOverload")
            .field("ty", &self.ty)
            .finish()
    }
}

// TODO: pull out into proper error type
#[derive(Debug)]
pub enum FnCallError<'reg> {
    NoneWithName,
    BadArgTypes,
    Ambiguous(Vec<&'reg FnType>),
}
