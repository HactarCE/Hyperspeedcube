use std::sync::Arc;

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
        loop_vars: Box<Spanned<Vec<Span>>>,
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
        args: Box<Spanned<Vec<Node>>>,
    },
    Fn(FnContents),

    // Literals
    NullLiteral,
    BoolLiteral(bool),
    NumberLiteral(f64),
    StringLiteral(Vec<StringSegment>),
    ListLiteral(Vec<Node>),
    MapLiteral(Vec<(Node, Node)>),

    // Parse error
    Error,
}
impl NodeContents {
    pub(crate) fn is_fn_call(&self) -> bool {
        matches!(self, Self::FnCall { .. })
    }

    pub(crate) fn kind_str(&self) -> &'static str {
        match self {
            NodeContents::Assign { .. } => "assignment statement",
            NodeContents::Export(_) => "'export' statement",
            NodeContents::FnDef { .. } => "named function definition",
            NodeContents::ImportAllFrom(_)
            | NodeContents::ImportFrom(_, _)
            | NodeContents::ImportAs(_, _)
            | NodeContents::Import(_) => "'import' statement",
            NodeContents::UseAllFrom(_) | NodeContents::UseFrom(_, _) => "'use' statement",
            NodeContents::Block(_) => "statement block",
            NodeContents::IfElse { .. } => "if statement",
            NodeContents::ForLoop { .. } => "'for' loop",
            NodeContents::WhileLoop { .. } => "'while' loop",
            NodeContents::Continue => "'continue' statement",
            NodeContents::Break => "'break' statement",
            NodeContents::Return(_) => "'return' statement",
            NodeContents::Ident(_) => "identifier",
            NodeContents::Op { .. } => "operator expression",
            NodeContents::FnCall { .. } => "function call expression",
            NodeContents::Paren(_) => "parenthetical expression",
            NodeContents::Access { .. } => "access expression",
            NodeContents::Index { .. } => "indexing expression",
            NodeContents::Fn(_) => "anonymous function expression",
            NodeContents::NullLiteral => "null literal",
            NodeContents::BoolLiteral(_) => "boolean literal",
            NodeContents::NumberLiteral(_) => "number literal",
            NodeContents::StringLiteral(_) => "string literal",
            NodeContents::ListLiteral(_) => "list literal",
            NodeContents::MapLiteral(_) => "map literal",
            NodeContents::Error => "error",
        }
    }
}

#[derive(Debug)]
pub struct FnContents {
    pub params: Vec<FnParam>,
    pub return_type: Option<Box<Node>>,
    pub body: Arc<Node>,
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
