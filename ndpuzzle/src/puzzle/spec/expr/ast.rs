pub struct ExprAst<'a> {
    pub span: &'a str,
    pub node: ExprAstNode<'a>,
}

pub enum ExprAstNode<'a> {
    Number(f32),
    Identifier(&'a str),
    FuncCall(&'a str, Vec<ExprAst<'a>>),
    Paren(Box<ExprAst<'a>>),
    Vector(Vec<ExprAst<'a>>),
    BinaryOp {
        lhs: Box<ExprAst<'a>>,
        op: BinaryOp,
        rhs: Box<ExprAst<'a>>,
    },
    UnaryOp {
        op: UnaryOp,
        arg: Box<ExprAst<'a>>,
    },
    Range {
        count: Box<ExprAst<'a>>,
        from: Box<ExprAst<'a>>,
        to: Box<ExprAst<'a>>,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,

    /// Property accessor.
    Accessor,

    /// Conjunction of transform conditions.
    Conj,

    /// Rotation operator.
    Rotate,
    /// Reflection operator.
    Reflect,
    /// Rotation angle adjustment operator.
    ByAngle,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Pos,
    Neg,
}
