use crate::{Span, Spanned};

pub type Node = Spanned<NodeContents>;

#[derive(Debug)]
pub enum NodeContents {
    // Declarations
    Assign {
        var: Box<Node>,
        ty: Option<Box<Node>>,
        assign_symbol: Span,
        value: Box<Node>,
    },
    Export(Box<Node>),
    FnDef {
        name: Span,
        contents: Box<FnContents>,
    },
    ImportAllFrom(ImportPath),
    ImportFrom(Vec<Span>, ImportPath),
    ImportAs(ImportPath, Span),
    Import(Span),
    UseAllFrom(Box<Node>),
    UseFrom(Vec<Span>, Box<Node>),

    // Control flow
    Block(Vec<Node>),
    IfElse {
        if_cases: Vec<(Box<Node>, Box<Node>)>,
        else_case: Option<Box<Node>>,
    },
    ForLoop {
        loop_vars: Vec<Span>,
        iterator: Box<Node>,
        body: Box<Node>,
    },
    WhileLoop {
        condition: Box<Node>,
        body: Box<Node>,
    },
    Continue,
    Break,
    Return(Option<Box<Node>>),

    // Expressions
    Ident(Span),
    Op {
        op: Span,
        args: Vec<Node>,
    },
    FnCall {
        func: Box<Node>,
        args: Vec<Node>,
    },
    Paren(Box<Node>),
    Access {
        obj: Box<Node>,
        field: Span,
    },
    Index {
        obj: Box<Node>,
        args: Vec<Node>,
    },
    Fn(FnContents),

    // Literals
    NullLiteral,
    BoolLiteral(bool),
    NumberLiteral(f64),
    StringLiteral(Vec<StringSegment>),
    ListLiteral(Vec<Node>),
    MapLiteral(Vec<(Node, Node)>),
}
impl NodeContents {
    pub(crate) fn is_fn_call(&self) -> bool {
        matches!(self, Self::FnCall { .. })
    }
}

#[derive(Debug)]
pub struct FnContents {
    pub params: Vec<FnParam>,
    pub return_type: Option<Box<Node>>,
    pub body: Box<Node>,
}

#[derive(Debug)]
pub struct FnParam {
    pub name: Span,
    pub ty: Option<Box<Node>>,
}

#[derive(Debug)]
pub enum StringSegment {
    Literal(Span),
    Char(char),
    Interpolation(Node),
}

#[derive(Debug)]
pub struct ImportPath(pub Vec<Span>);
