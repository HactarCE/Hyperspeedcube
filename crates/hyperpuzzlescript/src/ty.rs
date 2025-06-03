use std::borrow::Cow;
use std::collections::BTreeSet;
use std::fmt;

use itertools::Itertools;

use crate::Value;

/// Type in the language, which is a predicate on values.
///
/// These predicates may overlap.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[allow(missing_docs)]
pub enum Type {
    #[default]
    Any,
    Null,
    Bool,
    Num,
    Str,
    List(Option<Box<Type>>),
    Map,
    Fn,
    Type,

    // More specific predicates
    Int,
    Nat, // includes zero
    EmptyList,
    NonEmptyList(Option<Box<Type>>),

    Vec,

    EuclidPoint,
    EuclidTransform,
    EuclidPlane,
    EuclidRegion,
    EuclidBlade,

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

    Union(TypeUnion),

    Custom(&'static str),
}
impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Any => write!(f, "Any"),
            Type::Null => write!(f, "Null"),
            Type::Bool => write!(f, "Bool"),
            Type::Num => write!(f, "Num"),
            Type::Str => write!(f, "Str"),
            Type::List(None) => write!(f, "List"),
            Type::List(Some(inner)) => write!(f, "List[{inner}]"),
            Type::Map => write!(f, "Map"),
            Type::Fn => write!(f, "Fn"),
            Type::Type => write!(f, "Type"),
            Type::Int => write!(f, "Int"),
            Type::Nat => write!(f, "Nat"),
            Type::EmptyList => write!(f, "EmptyList"),
            Type::NonEmptyList(None) => write!(f, "NonEmptyList"),
            Type::NonEmptyList(Some(inner)) => write!(f, "NonEmptyList[{inner}]"),
            Type::Vec => write!(f, "Vec"),
            Type::EuclidPoint => write!(f, "euclid.Point"),
            Type::EuclidTransform => write!(f, "euclid.Transform"),
            Type::EuclidPlane => write!(f, "euclid.Plane"),
            Type::EuclidRegion => write!(f, "euclid.Region"),
            Type::EuclidBlade => write!(f, "euclid.Blade"),
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

            Type::Union(union) => match union.try_to_nullable_single_type() {
                Ok(nullable_single_type) => write!(f, "{nullable_single_type}?"),

                Err(types_list) => write!(f, "{}", types_list.iter().join(" | ")),
            },

            Type::Custom(name) => write!(f, "{name}"),
        }
    }
}
impl Type {
    /// Returns a superset of the union of `a` and `b`.
    pub fn unify(a: Type, b: Type) -> Type {
        match (a, b) {
            (a, b) if a == b => a,
            (a, b) if a.is_subtype_of(&b) => b,
            (b, a) if a.is_subtype_of(&b) => b,
            (Type::List(a_elem), Type::List(b_elem)) => Type::List(
                Option::zip(a_elem, b_elem)
                    .map(|(a_elem, b_elem)| Box::new(Type::unify(*a_elem, *b_elem))),
            ),
            (Type::Union(mut u), other) | (other, Type::Union(mut u)) => {
                u.insert(other);
                Type::Union(u)
            }
            (a, b) => {
                let mut u = TypeUnion::default();
                u.insert(a);
                u.insert(b);
                Type::Union(u)
            }
        }
    }

    /// Returns whether all values of type `self` also have type `other`.
    pub fn is_subtype_of(&self, other: &Type) -> bool {
        match (self, other) {
            (_, Type::Any) => true,

            (Type::List(self_inner), Type::List(other_inner))
            | (Type::NonEmptyList(self_inner), Type::List(other_inner))
            | (Type::NonEmptyList(self_inner), Type::NonEmptyList(other_inner)) => {
                match (&self_inner, &other_inner) {
                    (Some(a), Some(b)) => a.is_subtype_of(b),
                    _ => other_inner.is_none(),
                }
            }
            (Type::EmptyList, Type::List(_)) => true,

            (Type::Nat, Type::Int) | (Type::Nat, Type::Num) | (Type::Int, Type::Num) => true,

            (Type::Union(a), b) => a.to_vec().iter().all(|a| a.is_subtype_of(b)),
            (a, Type::Union(b)) => b.to_vec().iter().any(|b| a.is_subtype_of(b)),
            _ => self == other,
        }
    }
    /// Returns whether there is some value that has type `self` and type
    /// `other`.
    pub fn overlaps(&self, other: &Type) -> bool {
        match (self, other) {
            (Type::Any, _) | (_, Type::Any) => true,

            (Type::Num | Type::Int | Type::Nat, Type::Num | Type::Int | Type::Nat) => true,

            (Type::List(_) | Type::EmptyList, Type::List(_) | Type::EmptyList) => true,
            (
                Type::List(Some(a)) | Type::NonEmptyList(Some(a)),
                Type::List(Some(b)) | Type::NonEmptyList(Some(b)),
            ) => a.overlaps(&b),

            (Type::Union(u), a) | (a, Type::Union(u)) => u.to_vec().iter().any(|b| a.overlaps(b)),
            _ => self == other,
        }
    }

    /// Returns a union of this type with `Type::Null`.
    pub fn optional(self) -> Type {
        Self::unify(self, Type::Null)
    }
}
impl FromIterator<Type> for Type {
    fn from_iter<T: IntoIterator<Item = Type>>(iter: T) -> Self {
        iter.into_iter().reduce(Type::unify).unwrap_or_default()
    }
}
impl From<Type> for Cow<'_, Type> {
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
    /// Constructs a function type from an optional parameter list and a return
    /// type.
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

/// Union of list types.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct ListTypeUnion {
    empty: bool,
    non_empty: Option<Box<TypeUnion>>,
}
impl ListTypeUnion {
    fn any() -> Self {
        Self {
            empty: true,
            non_empty: Some(Box::new(TypeUnion::Any)),
        }
    }

    fn insert(&mut self, elem: Type) {
        self.insert_empty();
        self.insert_nonempty(elem);
    }
    fn insert_empty(&mut self) {
        self.empty = true;
    }
    fn insert_nonempty(&mut self, elem: Type) {
        self.non_empty.get_or_insert_default().insert(elem);
    }

    fn to_vec(&self) -> Vec<Type> {
        let get_empty_list_types = || match self.empty {
            true => vec![Type::EmptyList],
            false => vec![],
        };
        let list_type_fn = match self.empty {
            true => Type::List,
            false => Type::NonEmptyList,
        };

        match &self.non_empty {
            None => get_empty_list_types(),
            Some(elem_union) if elem_union.is_any() => vec![list_type_fn(None)],
            Some(elem_union) if elem_union.is_empty() => get_empty_list_types(),
            Some(elem_union) => elem_union
                .to_vec()
                .into_iter()
                .map(|ty| list_type_fn(Some(Box::new(ty))))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TypeUnion {
    Any,
    Specific {
        lists: ListTypeUnion,
        other_types: BTreeSet<Type>,
    },
}
impl Default for TypeUnion {
    fn default() -> Self {
        Self::Specific {
            lists: ListTypeUnion::default(),
            other_types: BTreeSet::new(),
        }
    }
}
impl TypeUnion {
    pub fn insert(&mut self, ty: Type) {
        let Self::Specific { lists, other_types } = self else {
            return;
        };

        match ty {
            Type::Any => *self = Self::Any,
            Type::List(None) => lists.insert(Type::Any),
            Type::List(Some(elem)) => lists.insert(*elem),
            Type::EmptyList => lists.insert_empty(),
            Type::NonEmptyList(None) => lists.insert_nonempty(Type::Any),
            Type::NonEmptyList(Some(elem)) => lists.insert_nonempty(*elem),
            Type::Union(union) => {
                for ty in union.to_vec() {
                    self.insert(ty);
                }
            }
            other => {
                other_types.insert(other);
            }
        }
    }

    pub fn to_vec(&self) -> Vec<Type> {
        match self {
            TypeUnion::Any => vec![Type::Any],
            TypeUnion::Specific { lists, other_types } => {
                itertools::chain(other_types.iter().cloned(), lists.to_vec()).collect()
            }
        }
    }

    pub fn is_any(&self) -> bool {
        matches!(self, TypeUnion::Any)
    }
    pub fn is_empty(&self) -> bool {
        match self {
            TypeUnion::Any => false,
            TypeUnion::Specific { lists, other_types } => {
                !lists.empty
                    && lists.non_empty.as_ref().is_none_or(|elem| elem.is_empty())
                    && other_types.is_empty()
            }
        }
    }

    /// If the union contains only `Null` and one other type, returns `Ok`
    /// containing that one other type. Otherwise returns `Err` containing all
    /// types (equivalent to [`Self::to_vec()`]).
    ///
    /// Examples:
    ///
    /// - empty union returns `Err`
    /// - `Int` returns `Err`
    /// - `List[Str]` returns `Err`
    /// - `Null | Int` returns `Ok(Int)`
    /// - `Null | List[Null | Str]` returns `Ok(List[Null | Str])`
    /// - `Null | List | Map` returns `Err`
    fn try_to_nullable_single_type(&self) -> Result<Type, Vec<Type>> {
        let types = self.to_vec();
        if types.len() == 2 && types.contains(&Type::Null) {
            let non_null_type = types
                .into_iter()
                .find(|t| *t != Type::Null)
                .unwrap_or(Type::Null);
            Ok(non_null_type)
        } else {
            Err(types)
        }
    }
}
