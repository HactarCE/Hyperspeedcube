use std::fmt;
use std::ops::Index;
use std::str::FromStr;
use std::sync::Arc;

use itertools::Itertools;

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
    /// - `fn ident(...) { ... }`
    FnDef {
        name: Span,
        contents: Box<FnContents>,
    },
    /// - `export * from expr`
    ExportAllFrom(Box<Node>),
    /// - `export ident1, ident2 as ident3 from expr`
    /// - `export (ident1, ident2 as ident3) from expr`
    ExportFrom(Vec<IdentAs>, Box<Node>),
    /// - `export ident`
    /// - `export expr as ident`
    ExportAs(IdentAs),
    /// - `export ident = expr`
    /// - `export ident: Type = expr`
    ExportAssign {
        name: Span,
        ty: Option<Box<Node>>,
        value: Box<Node>,
    },
    /// - `export fn ident(...) { ... }`
    ExportFnDef {
        name: Span,
        contents: Box<FnContents>,
    },
    /// - `use * from expr`
    UseAllFrom(Box<Node>),
    /// - `use ident1, ident2 as ident3 from expr`
    /// - `use (ident1, ident2 as ident3) from expr`
    UseFrom(Vec<IdentAs>, Box<Node>),

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
    With(SpecialVar, Box<Node>, Box<Node>),

    // Expressions
    Ident(Span),
    SpecialIdent(SpecialVar),
    Op {
        op: Span,
        args: Vec<Node>,
    },
    FnCall {
        func: Box<Node>,
        args: Vec<FnArg>,
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
    /// - `@dir1/dir2/file`
    /// - `@/relative_dir/file`
    /// - `@^/dir_in_parent/file`
    /// - `@^^/dir_in_grandparent/file`
    ///
    /// The `@` is included in the span.
    FilePath(Span),

    // Literals
    NullLiteral,
    BoolLiteral(bool),
    NumberLiteral(f64),
    StringLiteral(Vec<StringSegment>),
    ListLiteral(Vec<Node>),
    MapLiteral(Vec<MapEntry>),

    // Parse error
    Error,
}
impl NodeContents {
    pub(crate) fn kind_str(&self) -> &'static str {
        match self {
            NodeContents::Assign { .. } => "assignment statement",
            NodeContents::FnDef { .. } => "named function definition",
            NodeContents::ExportAllFrom(_)
            | NodeContents::ExportFrom(_, _)
            | NodeContents::ExportAs(_)
            | NodeContents::ExportAssign { .. }
            | NodeContents::ExportFnDef { .. } => "'export' statement",
            NodeContents::UseAllFrom(_) | NodeContents::UseFrom(_, _) => "'use' statement",
            NodeContents::Block(_) => "statement block",
            NodeContents::IfElse { .. } => "if statement",
            NodeContents::ForLoop { .. } => "'for' loop",
            NodeContents::WhileLoop { .. } => "'while' loop",
            NodeContents::Continue => "'continue' statement",
            NodeContents::Break => "'break' statement",
            NodeContents::Return(_) => "'return' statement",
            NodeContents::With(_, _, _) => "'with' block",
            NodeContents::Ident(_) => "identifier",
            NodeContents::SpecialIdent(_) => "special identifier",
            NodeContents::Op { .. } => "operator expression",
            NodeContents::FnCall { .. } => "function call expression",
            NodeContents::Paren(_) => "parenthetical expression",
            NodeContents::Access { .. } => "access expression",
            NodeContents::Index { .. } => "indexing expression",
            NodeContents::Fn(_) => "anonymous function expression",
            NodeContents::FilePath(_) => "file path",
            NodeContents::NullLiteral => "null literal",
            NodeContents::BoolLiteral(_) => "boolean literal",
            NodeContents::NumberLiteral(_) => "number literal",
            NodeContents::StringLiteral(_) => "string literal",
            NodeContents::ListLiteral(_) => "list literal",
            NodeContents::MapLiteral(_) => "map literal",
            NodeContents::Error => "error",
        }
    }

    pub(crate) fn as_list_splat(&self, ctx: &impl Index<Span, Output = str>) -> Option<&Node> {
        match self {
            NodeContents::Op { op, args } if &ctx[*op] == "*" => args.iter().exactly_one().ok(),
            _ => None,
        }
    }
    pub(crate) fn as_map_splat(&self, ctx: &impl Index<Span, Output = str>) -> Option<&Node> {
        match self {
            NodeContents::Op { op, args } if &ctx[*op] == "**" => args.iter().exactly_one().ok(),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum MapEntry {
    KeyValue {
        key: Node,
        ty: Option<Box<Node>>,
        value: Option<Box<Node>>,
    },
    Splat {
        span: Span,
        values: Node,
    },
}

#[derive(Debug)]
pub struct FnArg {
    pub name: Option<Span>,
    pub value: Box<Node>,
}

#[derive(Debug)]
pub struct FnContents {
    pub params: Vec<FnParam>,
    pub return_type: Option<Box<Node>>,
    pub body: Arc<Node>,
}

#[derive(Debug)]
pub enum FnParam {
    /// - `ident`
    /// - `ident: Type`
    /// - `ident = expr`
    /// - `ident: Type = expr`
    Param {
        name: Span,
        ty: Option<Box<Node>>,
        default: Option<Box<Node>>,
    },
    /// - `*ident`
    SeqSplat(Span),
    /// - `*`
    SeqEnd(Span),
    /// - `**ident`
    NamedSplat(Span),
}

#[derive(Debug)]
pub enum StringSegment {
    Literal(Span),
    Char(char),
    Interpolation(Node),
}

/// AST node for `ident1 as ident2` syntax structure (where `as ident2` is
/// optional).
#[derive(Debug)]
pub struct IdentAs {
    /// Target to get.
    pub target: Span,
    /// Alias to import/export as.
    pub alias: Option<Span>,
}
impl IdentAs {
    /// Returns `alias` or, if it is `None`, `target`.
    pub fn alias(&self) -> Span {
        self.alias.unwrap_or(self.target)
    }
}

/// Special variable, which is inherited via the call graph instead of scope.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SpecialVar {
    /// Number of dimensions of the space.
    Ndim,
    /// Symmetry group to apply for puzzle operations.
    Sym,
}
impl fmt::Display for SpecialVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpecialVar::Ndim => write!(f, "#ndim"),
            SpecialVar::Sym => write!(f, "#sym"),
        }
    }
}
impl FromStr for SpecialVar {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "#ndim" => Ok(Self::Ndim),
            "#sym" => Ok(Self::Sym),
            _ => Err(()),
        }
    }
}
