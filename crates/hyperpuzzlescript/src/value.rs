use std::borrow::Cow;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use arcstr::Substr;
use ecow::EcoString;
use hypermath::{Vector, VectorRef};
use indexmap::IndexMap;
use itertools::Itertools;

use crate::{
    Error, EvalCtx, FnType, FullDiagnostic, Key, ND_EUCLID, Result, Scope, Span, Spanned,
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

            _ => Err(Error::InvalidComparison(
                Box::new((self.ty(), self.span)),
                Box::new((other.ty(), other.span)),
            )
            .at(span)),
        }
    }

    /// Returns a type error saying that this value has the wrong type.
    pub fn type_error(&self, expected: Type) -> FullDiagnostic {
        self.multi_type_error(vec![expected])
    }
    /// Returns a type error saying that this value has the wrong type.
    pub fn multi_type_error(&self, expected: Vec<Type>) -> FullDiagnostic {
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
        match (&self.data, expected) {
            (ValueData::List(list), Type::List(expected_inner_ty)) => {
                list.iter().all(|elem| elem.is_type(expected_inner_ty))
            }
            (ValueData::Map(map), Type::Map(expected_inner_ty)) => {
                map.values().all(|value| value.is_type(expected_inner_ty))
            }
            (ValueData::Fn(f), Type::Fn(fn_type)) => f.any_overload_is_subtype_of(fn_type),
            _ => self.data.ty().is_subtype_of(expected),
        }
    }

    /// Returns the list, or an error if this isn't a list.
    pub fn as_list(&self) -> Result<&Arc<Vec<Value>>> {
        match &self.data {
            ValueData::List(l) => Ok(l),
            _ => Err(self.type_error(Type::List(Default::default()))),
        }
    }
    /// Returns the list, or an error if this isn't a list.
    pub fn into_list(self) -> Result<Arc<Vec<Value>>> {
        match self.data {
            ValueData::List(values) => Ok(values),
            _ => Err(self.type_error(Type::List(Default::default()))),
        }
    }
    /// Returns the map, or an error if this isn't a map.
    pub fn as_map(&self) -> Result<&Arc<IndexMap<Key, Value>>> {
        match &self.data {
            ValueData::Map(m) => Ok(m),
            _ => Err(self.type_error(Type::Map(Default::default()))),
        }
    }
    /// Returns the map, or an error if this isn't a map.
    pub fn into_map(self) -> Result<Arc<IndexMap<Key, Value>>> {
        match self.data {
            ValueData::Map(m) => Ok(m),
            _ => Err(self.type_error(Type::Map(Default::default()))),
        }
    }
    /// Returns the function, or an error if this isn't a function.
    pub fn as_func(&self) -> Result<&Arc<FnValue>> {
        match &self.data {
            ValueData::Fn(f) => Ok(f),
            _ => Err(self.type_error(Type::Fn(Default::default()))),
        }
    }
    /// Returns the function. If this value wasn't a function before, this
    /// function will make it become one with the given name.
    pub fn as_func_mut(&mut self, span: Span, name: Option<Substr>) -> &mut FnValue {
        if !matches!(self.data, ValueData::Fn(_)) {
            *self = ValueData::Fn(Arc::new(FnValue::new(name))).at(span);
        }
        match &mut self.data {
            ValueData::Fn(f) => Arc::make_mut(f),
            _ => unreachable!(),
        }
    }
    /// Returns the string, or an error if this isn't a string. This does not
    /// convert other types to a string.
    pub fn as_str(&self) -> Result<&EcoString> {
        match &self.data {
            ValueData::Str(s) => Ok(s),
            _ => Err(self.type_error(Type::Str)),
        }
    }
    /// Returns the boolean, or an error if this isn't a boolean. This does not
    /// convert other types to a boolean.
    pub(crate) fn as_bool(&self) -> Result<bool> {
        match &self.data {
            ValueData::Bool(b) => Ok(*b),
            _ => Err(self.type_error(Type::Bool)),
        }
    }
    /// Returns the number, or an error if this isn't a number.
    pub(crate) fn as_num(&self) -> Result<f64> {
        match &self.data {
            ValueData::Num(n) => Ok(*n),
            _ => Err(self.type_error(Type::Num)),
        }
    }
    /// Returns the number as an `i64`, or an error if this isn't an integer
    /// that fits within an `i64`.
    pub(crate) fn as_int(&self) -> Result<i64> {
        let n = self.as_num()?;
        hypermath::to_approx_integer(n).ok_or(Error::ExpectedInteger(n).at(self.span))
    }
    /// Returns the number as an `u64`, or an error if this isn't an integer
    /// that fits within an `u64`.
    pub(crate) fn as_uint(&self) -> Result<u64> {
        let n = self.as_num()?;
        hypermath::to_approx_unsigned_integer(n)
            .ok_or(Error::ExpectedNonnegativeInteger(n).at(self.span))
    }
    /// Returns the integer as a `u8`, or an error if this isn't an integer that
    /// fits with in a `u8`.
    pub(crate) fn as_u8(&self) -> Result<u8> {
        let i = self.as_int()?;
        i.try_into().map_err(|_| {
            let bounds = Some((u8::MIN as i64, u8::MAX as i64));
            Error::IndexOutOfBounds { got: i, bounds }.at(self.span)
        })
    }

    /// Converts the value to a vector.
    pub(crate) fn to_vector(&self) -> Result<Vector> {
        match &self.data {
            ValueData::Vec(v) => Ok(v.clone()),
            _ => Err(self.multi_type_error(vec![Type::Vec])),
        }
    }
    /// Converts the value to a PGA blade.
    pub(crate) fn to_pga_blade(&self) -> Result<hypermath::pga::Blade> {
        match &self.data {
            ValueData::Vec(v) => Ok(hypermath::pga::Blade::from_vector(v)),
            ValueData::EuclidBlade(b) => Ok(b.clone()),
            _ => Err(self.multi_type_error(vec![Type::EuclidBlade])),
        }
    }

    pub(crate) fn as_opt(&self) -> Option<&Self> {
        (!self.is_null()).then_some(self)
    }

    /// Converts the value to an integer and then uses it to index a
    /// double-ended collection.
    ///
    /// This may be take O(n) time with respect to the size of the collection,
    /// but many double-ended iterators in Rust have O(1) implementations of
    /// `.nth()` and `.nth_back()` so it is often performant.
    pub(crate) fn index_double_ended<I: IntoIterator>(
        &self,
        iter: I,
        get_len: impl FnOnce() -> usize,
    ) -> Result<I::Item>
    where
        I::IntoIter: DoubleEndedIterator,
    {
        self.index(
            iter.into_iter(),
            |mut it: I::IntoIter, i| it.nth(i),
            Some(|mut it: I::IntoIter, i| it.nth_back(i)),
            get_len,
        )
    }

    /// Converts the value to an integer and then uses it to index a collection.
    ///
    /// - `get_front` should return the `i`th value from the front of the
    ///   collection (starting at zero)
    /// - `get_back` should return the `i`th value from the back of the
    ///   collection (starting at zero).
    /// - `get_back` should be `None` if the collection does not support
    ///   indexing from the back.
    /// - `get_len` should return the length of the collection.
    pub(crate) fn index<C, T>(
        &self,
        collection: C,
        get_front: impl FnOnce(C, usize) -> Option<T>,
        get_back: Option<impl FnOnce(C, usize) -> Option<T>>,
        get_len: impl FnOnce() -> usize,
    ) -> Result<T> {
        let allow_negatives = get_back.is_some();
        let i = self.as_int()?;
        match i {
            0.. => get_front(collection, i.try_into().unwrap_or(usize::MAX)),
            ..0 => get_back.and_then(|f| f(collection, (-i - 1).try_into().unwrap_or(usize::MAX))),
        }
        .ok_or_else(|| {
            Error::IndexOutOfBounds {
                got: i,
                bounds: (|| {
                    let max = get_len().checked_sub(1)? as i64;
                    let min = if allow_negatives { -max - 1 } else { 0 };
                    Some((min, max))
                })(),
            }
            .at(self.span)
        })
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
    List(Arc<Vec<Value>>), // TODO: use rpds::Vector
    /// Copy-on-write dictionary with string keys.
    Map(Arc<IndexMap<Key, Value>>), // TODO: use rpds::RedBlackTreeMap
    /// Function with a copy-on-write list of overloads.
    Fn(Arc<FnValue>), // TODO: use rpds::Vector

    /// N-dimensional vector of numbers. Unused dimensions are zero.
    Vec(Vector),

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
            Self::EuclidBlade(_) => Type::EuclidBlade,
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
}
impl From<()> for ValueData {
    fn from((): ()) -> Self {
        ValueData::Null
    }
}
impl From<Value> for ValueData {
    fn from(value: Value) -> Self {
        value.data
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
impl From<u8> for ValueData {
    fn from(value: u8) -> Self {
        ValueData::Num(value as f64)
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
impl From<String> for ValueData {
    fn from(value: String) -> Self {
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
impl<K: Into<Key>> FromIterator<(K, Value)> for ValueData {
    fn from_iter<T: IntoIterator<Item = (K, Value)>>(iter: T) -> Self {
        Self::Map(Arc::new(
            iter.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        ))
    }
}
impl<V: Into<ValueData>> From<Vec<Spanned<V>>> for ValueData {
    fn from(value: Vec<Spanned<V>>) -> Self {
        Self::List(Arc::new(
            value
                .into_iter()
                .map(|(data, span)| Value {
                    data: data.into(),
                    span,
                })
                .collect(),
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
impl From<hypermath::pga::Motor> for ValueData {
    fn from(value: hypermath::pga::Motor) -> Self {
        Self::EuclidTransform(value)
    }
}
impl From<hypermath::Hyperplane> for ValueData {
    fn from(value: hypermath::Hyperplane) -> Self {
        Self::EuclidPlane(Box::new(value))
    }
}
impl From<&Scope> for ValueData {
    fn from(value: &Scope) -> Self {
        Self::Map(Arc::new(
            value
                .names
                .lock()
                .iter()
                .map(|(name, value)| (name.clone(), value.clone()))
                .sorted_by(|(name1, _), (name2, _)| name1.cmp(name2))
                .collect(),
        ))
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
    /// Calls the function.
    pub fn call(
        &self,
        call_span: Span,
        fn_span: Span,
        ctx: &mut EvalCtx<'_>,
        args: Vec<Value>,
        kwargs: IndexMap<Key, Value>,
    ) -> Result<Value> {
        let overload = self.get_overload(fn_span, &args)?;

        let fn_scope = match &overload.parent_scope {
            Some(parent) => Cow::Owned(Scope::new_closure(Arc::clone(parent), self.name.clone())),
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
    pub call: Arc<
        dyn Send + Sync + Fn(&mut EvalCtx<'_>, Vec<Value>, IndexMap<Key, Value>) -> Result<Value>,
    >,
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
