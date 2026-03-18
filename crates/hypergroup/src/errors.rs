use hypuz_util::ti::IndexOverflow;

use crate::GroupElementId;

/// Result type returned by group construction operations.
pub type GroupResult<T> = Result<T, GroupError>;

/// Error that can occur during group construction.
#[expect(missing_docs)]
#[derive(thiserror::Error, Debug, Clone)]
pub enum GroupError {
    #[error("overflow ({0})")]
    Overflow(#[from] IndexOverflow),
    #[error("group is too high-dimensional")]
    TooHighDimensional,

    #[error("incomplete group structure")]
    IncompleteGroupStructure,
    #[error("bad group structure")]
    BadGroupStructure,
    #[error("bad inverse; inverse of {0} is {1} but inverse of {1} is {2}")]
    BadInverse(GroupElementId, GroupElementId, GroupElementId),

    #[error("coxeter-dynkin diagram is hyperbolic")]
    HyperbolicCD,
    #[error("coxeter-dynkin diagram is euclidean")]
    EuclideanCD,
    #[error("invalid coxeter-dynkin diagram")]
    BadCD,

    #[error("bad motor")]
    BadMotor,
}
