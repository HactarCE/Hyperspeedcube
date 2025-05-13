/// Type in the language.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    #[default]
    Any,
    Null,
    Bool,
    Number,
    String,
    List(Box<Type>),
    Map(Box<Type>),
    Fn(Box<FnType>),
    Type,

    Vector,

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
impl Type {
    fn unify(a: Type, b: Type) -> Type {
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

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct FnType {
    pub params: Option<Vec<Type>>,
    pub ret: Type,
}
impl FnType {
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
}
