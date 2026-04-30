use super::*;

/// Parameter for a puzzle generator.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratorParam {
    /// Human-friendly name.
    pub name: String,
    /// Parameter type.
    pub ty: GeneratorParamType,
    /// Default value.
    pub default: CatalogIdValue,
}

impl GeneratorParam {
    /// Converts a catalog ID value into a typed value for this parameter, or
    /// returns an error if it is invalid.
    pub fn typed_value(
        &self,
        arg: CatalogIdValue,
    ) -> Result<TypedCatalogIdValue, GeneratorParamError> {
        match &self.ty {
            GeneratorParamType::Bool => arg.to_bool().map(TypedCatalogIdValue::Bool),
            GeneratorParamType::Int { .. } => arg.to_int().map(TypedCatalogIdValue::Int),
            GeneratorParamType::Puzzle { .. } => arg.into_id().map(TypedCatalogIdValue::Id),
            GeneratorParamType::List(inner) => arg
                .into_list()
                .and_then(|l| l.into_iter().map(|e| inner.typed_value(&e)).try_collect())
                .map(TypedCatalogIdValue::List),
        }
        .map_err(|inner| GeneratorParamError {
            param: self.clone(),
            inner,
        })
    }
}

/// Type of a parameter for a puzzle generator.
#[derive(Debug, Clone, PartialEq)]
pub enum GeneratorParamType {
    /// Boolean.
    Bool,
    /// Integer.
    Int {
        /// Minimum value (inclusive).
        min: i64,
        /// Maximum value (inclusive).
        max: i64,
    },
    /// Puzzle ID with a menu name.
    Puzzle {
        /// Puzzle menu ID.
        menu: String,
    },
    /// List of parameters.
    ///
    /// This must be the last parameter.
    List(Box<GeneratorParamType>),
}

impl fmt::Display for GeneratorParamType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GeneratorParamType::Bool => write!(f, "true or false"),
            GeneratorParamType::Int { min, max } => write!(f, "integer ({min} to {max})"),
            GeneratorParamType::Puzzle { menu } => write!(f, "puzzle from {menu:?} menu"),
            GeneratorParamType::List(inner) => write!(f, "list of {inner}"),
        }
    }
}

impl GeneratorParamType {
    /// Converts a catalog ID value into a typed value for this parameter, or
    /// returns an error if it is invalid.
    pub fn typed_value(&self, arg: &CatalogIdValue) -> Result<TypedCatalogIdValue, CatalogIdError> {
        match self {
            GeneratorParamType::Bool => arg.to_bool().map(TypedCatalogIdValue::Bool),
            GeneratorParamType::Int { .. } => arg.to_int().map(TypedCatalogIdValue::Int),
            GeneratorParamType::Puzzle { .. } => arg.clone().into_id().map(TypedCatalogIdValue::Id),
            GeneratorParamType::List(inner) => arg
                .clone()
                .into_list()
                .and_then(|l| l.into_iter().map(|e| inner.typed_value(&e)).try_collect())
                .map(TypedCatalogIdValue::List),
        }
    }
}

/// Value of a parameter for a puzzle generator.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypedCatalogIdValue {
    /// Catalog ID.
    Id(CatalogId),
    /// Boolean.
    Bool(bool),
    /// Integer.
    Int(i64),
    /// List of values.
    List(Vec<TypedCatalogIdValue>),
}

impl fmt::Display for TypedCatalogIdValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypedCatalogIdValue::Id(id) => write!(f, "{id}"),
            TypedCatalogIdValue::Bool(b) => write!(f, "{b}"),
            TypedCatalogIdValue::Int(i) => write!(f, "{i}"),
            TypedCatalogIdValue::List(l) => {
                write!(f, "[")?;
                let mut is_first = true;
                for elem in l {
                    if !std::mem::take(&mut is_first) {
                        write!(f, ",")?;
                    }
                    write!(f, "{elem}")?;
                }
                write!(f, "]")?;
                Ok(())
            }
        }
    }
}

impl From<TypedCatalogIdValue> for CatalogIdValue {
    fn from(value: TypedCatalogIdValue) -> Self {
        value.into_untyped()
    }
}

impl TypedCatalogIdValue {
    /// Converts a [`TypedCatalogIdValue`] to a [`CatalogIdValue`], which loses
    /// the type information.
    pub fn into_untyped(self) -> CatalogIdValue {
        match self {
            Self::Id(id) => id.into(),
            Self::Bool(b) => b.into(),
            Self::Int(i) => i.into(),
            Self::List(l) => l.into_iter().map(|e| e.into()).collect_vec().into(),
        }
    }
}

/// Error encountered when parsing a generator parameter.
#[derive(Debug)]
pub struct GeneratorParamError {
    /// Parameter requirements.
    pub param: GeneratorParam,
    /// Underlying error.
    pub inner: CatalogIdError,
}

impl fmt::Display for GeneratorParamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { param, inner } = self;
        let GeneratorParam { name, ty, .. } = param;
        write!(f, "bad value for param {name:?} (expected {ty}): {inner}")
    }
}

impl std::error::Error for GeneratorParamError {}
