use std::borrow::Cow;
use std::fmt;

use crate::Value;

/// Type in the language.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum Type {
    #[default]
    Any,
    Null,
    Bool,
    Num,
    Str,
    List(Box<Type>),
    Map(Box<Type>),
    Fn(Box<FnType>),

    Vec,

    EuclidPoint,
    EuclidTransform,
    EuclidPlane,
    EuclidRegion,

    Cga2dBlade1,
    Cga2dBlade2,
    Cga2dBlade3,
    Cga2dAntiscalar,
    Cga2dRegion,

    Color,
    Axis,
    Twist,

    AxisSystem,
    TwistSystem,
    Puzzle,
}
impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Any => write!(f, "Any"),
            Type::Null => write!(f, "Null"),
            Type::Bool => write!(f, "Bool"),
            Type::Num => write!(f, "Num"),
            Type::Str => write!(f, "Str"),
            Type::List(inner) => {
                write!(f, "List")?;
                if **inner != Type::Any {
                    write!(f, "[{inner}]")?;
                }
                Ok(())
            }
            Type::Map(inner) => {
                write!(f, "Map")?;
                if **inner != Type::Any {
                    write!(f, "[{inner}]")?;
                }
                Ok(())
            }
            Type::Fn(fn_type) => write!(f, "{fn_type}"),
            Type::Vec => write!(f, "Vec"),
            Type::EuclidPoint => write!(f, "euclid.Point"),
            Type::EuclidTransform => write!(f, "euclid.Transform"),
            Type::EuclidPlane => write!(f, "euclid.Plane"),
            Type::EuclidRegion => write!(f, "euclid.Region"),
            Type::Cga2dBlade1 => write!(f, "cga2d.Blade1"),
            Type::Cga2dBlade2 => write!(f, "cga2d.Blade2"),
            Type::Cga2dBlade3 => write!(f, "cga2d.Blade3"),
            Type::Cga2dAntiscalar => write!(f, "cga2d.Antiscalar"),
            Type::Cga2dRegion => write!(f, "cga2d.Region"),
            Type::Color => write!(f, "Color"),
            Type::Axis => write!(f, "Axis"),
            Type::Twist => write!(f, "Twist"),
            Type::AxisSystem => write!(f, "AxisSystem"),
            Type::TwistSystem => write!(f, "TwistSystem"),
            Type::Puzzle => write!(f, "Puzzle"),
        }
    }
}
impl Type {
    /// Returns a superset of the union of `a` and `b`.
    pub fn unify(a: Type, b: Type) -> Type {
        match (a, b) {
            (a, b) if a == b => a,
            (Type::List(a_elem), Type::List(b_elem)) => {
                Type::List(Box::new(Type::unify(*a_elem, *b_elem)))
            }
            (Type::Map(a_elem), Type::Map(b_elem)) => {
                Type::Map(Box::new(Type::unify(*a_elem, *b_elem)))
            }
            (Type::Fn(a_fn), Type::Fn(b_fn)) => Type::Fn(Box::new(FnType::unify(*a_fn, *b_fn))),
            _ => Type::Any,
        }
    }

    /// Returns whether all values of type `self` also have type `other`.
    pub fn is_subtype_of(&self, other: &Type) -> bool {
        match (self, other) {
            (_, Type::Any) => true,
            (Type::List(self_inner), Type::List(other_inner))
            | (Type::Map(self_inner), Type::Map(other_inner)) => {
                self_inner.is_subtype_of(other_inner)
            }
            (Type::Fn(self_inner), Type::Fn(other_inner)) => self_inner.is_subtype_of(other_inner),
            _ => self == other,
        }
    }
    /// Returns whether there is any value that has type `self` and type
    /// `other`.
    pub fn overlaps(&self, other: &Type) -> bool {
        match (self, other) {
            (Type::Any, _) | (_, Type::Any) => true,
            (Type::List(_), Type::List(_)) | (Type::Map(_), Type::Map(_)) => true,
            (Type::Fn(self_inner), Type::Fn(other_inner)) => {
                self_inner.might_be_subtype_of(other_inner)
            }
            _ => self == other,
        }
    }
}
impl FromIterator<Type> for Type {
    fn from_iter<T: IntoIterator<Item = Type>>(iter: T) -> Self {
        iter.into_iter().reduce(Type::unify).unwrap_or_default()
    }
}
impl From<FnType> for Type {
    fn from(value: FnType) -> Self {
        Type::Fn(Box::new(value))
    }
}
impl<'a> From<Type> for Cow<'a, Type> {
    fn from(value: Type) -> Self {
        Cow::Owned(value)
    }
}
impl<'a> From<&'a Type> for Cow<'a, Type> {
    fn from(value: &'a Type) -> Self {
        Cow::Borrowed(value)
    }
}

/// Function type.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct FnType {
    /// Parameter types, or `None` if the number of parameters is unknown.
    pub params: Option<Vec<Type>>,
    /// Return type.
    pub ret: Type,
}
impl fmt::Display for FnType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fn(")?;

        if let Some(param_types) = &self.params {
            let mut is_first = true;
            for param_ty in param_types {
                if is_first {
                    is_first = false;
                } else {
                    write!(f, ", ")?;
                }
                write!(f, "{param_ty}")?;
            }
        } else {
            write!(f, "...")?;
        }

        write!(f, ")")?;

        if self.ret != Type::Any {
            write!(f, " -> {}", self.ret)?;
        }

        Ok(())
    }
}
impl FnType {
    pub fn new(params: Option<Vec<Type>>, ret: Type) -> Self {
        Self { params, ret }
    }

    fn unify(a: FnType, b: FnType) -> FnType {
        FnType {
            params: Option::zip(a.params, b.params)
                .filter(|(a_params, b_params)| a_params.len() == b_params.len())
                .map(|(a_params, b_params)| {
                    std::iter::zip(a_params, b_params)
                        .map(|(a_param, b_param)| Type::unify(a_param, b_param))
                        .collect()
                }),
            ret: Type::unify(a.ret, b.ret),
        }
    }

    /// Returns whether this function might conflict with `other` if they were
    /// both overloads assigned to the same name.
    pub fn might_conflict_with(&self, other: &FnType) -> bool {
        // If either function is missing arg types, then there is definitely a
        // conflict.
        let Some(self_params) = &self.params else {
            return true;
        };
        let Some(other_params) = &other.params else {
            return true;
        };

        // If the parameter lists have different lengths, there is definitely
        // NOT a conflict.
        if self_params.len() != other_params.len() {
            return false;
        }

        std::iter::zip(self_params, other_params)
            .all(|(self_param, other_param)| self_param.overlaps(other_param))
    }

    /// Returns whether `self` is definitely a subtype of `other`.
    pub fn is_subtype_of(&self, other: &FnType) -> bool {
        self.ret.is_subtype_of(&other.ret)
            && match (&self.params, &other.params) {
                (Some(self_params), Some(other_params)) => {
                    // contravariance!
                    self_params.len() == other_params.len()
                        && std::iter::zip(self_params, other_params)
                            .all(|(self_param, other_param)| other_param.is_subtype_of(self_param))
                }
                (Some(_self_params), None) => true,
                (None, _maybe_other_params) => false,
            }
    }
    /// Returns whether `self` might be a subtype of `other`.
    pub fn might_be_subtype_of(&self, other: &FnType) -> bool {
        self.ret.overlaps(&other.ret)
            && match (&self.params, &other.params) {
                (Some(self_params), Some(other_params)) => {
                    self_params.len() == other_params.len()
                        && std::iter::zip(self_params, other_params)
                            .all(|(self_param, other_param)| other_param.overlaps(self_param))
                }
                _ => true,
            }
    }

    /// Returns whether this function might take `args` as arguments.
    pub fn might_take(&self, arg_types: &[Type]) -> bool {
        match &self.params {
            Some(params) => {
                params.len() == arg_types.len()
                    && std::iter::zip(params, arg_types).all(|(param, arg)| arg.overlaps(param))
            }
            None => true,
        }
    }
    /// Returns whether this function would definitely take `args` as arguments.
    pub fn would_take(&self, args: &[Value]) -> bool {
        match &self.params {
            Some(params) => {
                params.len() == args.len()
                    && std::iter::zip(params, args).all(|(param, arg)| arg.is_type(param))
            }
            None => true,
        }
    }
}
