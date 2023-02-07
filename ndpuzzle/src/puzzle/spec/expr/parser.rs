use anyhow::{anyhow, bail, Result};
use enum_iterator::Sequence;
use regex::Regex;

use super::ast::*;

lazy_static! {
    /// Regex that matches any valid floating-point literal (and many invalid
    /// ones) at the start of a string.
    ///
    /// [+\-]?[\d\.][\w\.+\-]*
    /// [+\-]?                      optional sign
    ///       [\d\.]                digit or decimal point
    ///             [\w\.+\-]*      letters, digits, decimal points, and signs;
    ///                             This matches lots of invalid floats but
    ///                             won't match anything else we care about.
    static ref GENEROUS_FLOAT_REGEX: Regex = Regex::new(r#"^[+\-]?[\d\.][\w\.+\-]*"#).unwrap();

    /// Regex that matches an identifier at the start of a string.
    static ref IDENTIFIER_REGEX: Regex= Regex::new(r#"^[a-zA-Z_]\w*"#).unwrap();

    /// Regex that matches an arrow of any length at the start of a string.
    static ref ARROW: Regex = Regex::new(r#"^-+>"#).unwrap();
}

pub fn parse_expression<'a>(input: &'a str) -> Result<ExprAst<'a>> {
    Parser { input, cursor: 0 }.parse_expr()
}

#[derive(Debug, Default, Sequence, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum OpPrecedence {
    #[default]
    TransformConjunction,
    Transform,

    Range,

    AddSub,
    MulDiv,
    Pow,

    Suffix,

    Prefix,

    Atom,
}

#[derive(Debug, Clone)]
struct Parser<'a> {
    input: &'a str,
    cursor: usize,
}
impl<'a> Parser<'a> {
    fn rest(&self) -> &'a str {
        &self.input[self.cursor..]
    }
    fn span_from(&self, start: usize) -> &'a str {
        &self.input[start..self.cursor]
    }
    fn span_between(start: &'a str, rest: &'a str) -> &'a str {
        let len = rest.len() - start.len();
        &start[..len]
    }

    fn parse_expr(&mut self) -> Result<ExprAst<'a>> {
        self.parse_expr_of(OpPrecedence::first().unwrap())
    }

    fn parse_expr_of(&mut self, prec: OpPrecedence) -> Result<ExprAst<'a>> {
        match prec {
            OpPrecedence::TransformConjunction => {
                self.parse_left_associative_op_expr(prec, &[("&", BinaryOp::Conj)])
            }
            OpPrecedence::Transform => self.parse_left_associative_op_expr(
                prec,
                &[
                    ("->", BinaryOp::Rotate),
                    ("|", BinaryOp::Reflect),
                    ("by", BinaryOp::ByAngle),
                ],
            ),

            OpPrecedence::Range => self.parse_range(prec),

            OpPrecedence::AddSub => self.parse_left_associative_op_expr(
                prec,
                &[("+", BinaryOp::Add), ("-", BinaryOp::Sub)],
            ),
            OpPrecedence::MulDiv => self.parse_left_associative_op_expr(
                prec,
                &[("*", BinaryOp::Mul), ("/", BinaryOp::Div)],
            ),
            OpPrecedence::Pow => {
                self.parse_right_associative_op_expr(prec, &[("^", BinaryOp::Pow)])
            }

            OpPrecedence::Suffix => {
                self.parse_left_associative_op_expr(prec, &[(".", BinaryOp::Accessor)])
            }

            OpPrecedence::Prefix => {
                self.parse_unary_prefix_op_expr(prec, &[("+", UnaryOp::Pos), ("-", UnaryOp::Neg)])
            }

            OpPrecedence::Atom => self.parse_atom(),
        }
    }

    fn parse_range(&mut self, prec: OpPrecedence) -> Result<ExprAst<'a>> {
        let start = self.cursor;
        let next_prec = prec.next().unwrap();

        let ret = self.parse_expr_of(next_prec)?;
        if self.accept("from") {
            let count = Box::new(ret);
            let from = Box::new(self.parse_expr()?);
            self.expect("to")?;
            let to = Box::new(self.parse_expr()?);
            Ok(ExprAst {
                span: self.span_from(start),
                node: ExprAstNode::Range { count, from, to },
            })
        } else {
            Ok(ret)
        }
    }

    fn parse_left_associative_op_expr(
        &mut self,
        prec: OpPrecedence,
        ops: &[(&str, BinaryOp)],
    ) -> Result<ExprAst<'a>> {
        let start = self.cursor;
        let next_prec = prec.next().unwrap();

        let mut ret = self.parse_expr_of(next_prec)?;
        loop {
            let Some(op) = self.accept_any(ops.iter().copied()) else {
                break Ok(ret);
            };

            let lhs = Box::new(ret);
            let rhs = Box::new(self.parse_expr_of(next_prec)?);

            ret = ExprAst {
                span: self.span_from(start),
                node: ExprAstNode::BinaryOp { lhs, op, rhs },
            }
        }
    }

    fn parse_right_associative_op_expr(
        &mut self,
        prec: OpPrecedence,
        ops: &[(&str, BinaryOp)],
    ) -> Result<ExprAst<'a>> {
        let start = self.cursor;
        let next_prec = prec.next().unwrap();

        let ret = self.parse_expr_of(next_prec)?;

        let Some(op) = self.accept_any(ops.iter().copied()) else {
            return Ok(ret);
        };

        let lhs = Box::new(ret);
        let rhs = Box::new(self.parse_expr_of(next_prec)?);
        Ok(ExprAst {
            span: self.span_from(start),
            node: ExprAstNode::BinaryOp { lhs, op, rhs },
        })
    }

    fn parse_unary_prefix_op_expr(
        &mut self,
        prec: OpPrecedence,
        ops: &[(&str, UnaryOp)],
    ) -> Result<ExprAst<'a>> {
        let next_prec = prec.next().unwrap();

        self.try_spanned(|this| {
            this.accept_any(ops.iter().copied()).map(|op| {
                let arg = Box::new(this.parse_expr_of(next_prec)?);
                Ok(ExprAstNode::UnaryOp { op, arg })
            })
        })
        .unwrap_or_else(|| self.parse_expr_of(next_prec))
    }

    fn parse_expr_list(&mut self) -> Result<Vec<ExprAst<'a>>> {
        let mut list = vec![];
        loop {
            if self.peek(")") || self.peek("]") {
                break Ok(list);
            }
            list.push(self.parse_expr()?);
            if self.accept(",") {
                continue;
            } else {
                break Ok(list);
            }
        }
    }

    fn parse_atom(&mut self) -> Result<ExprAst<'a>> {
        None.or_else(|| self.accept_paren_grouping())
            .or_else(|| self.accept_vector_literal())
            .or_else(|| self.accept_numeric_literal())
            .or_else(|| self.accept_func_call())
            .or_else(|| self.accept_identifier())
            .unwrap_or_else(|| bail!("unknown symbol at start of {:?}", self.rest()))
    }

    fn accept_paren_grouping(&mut self) -> Option<Result<ExprAst<'a>>> {
        self.try_spanned(|this| {
            this.accept("(").then(|| {
                let inner = this.parse_expr()?;
                this.expect(")")?;
                Ok(ExprAstNode::Paren(Box::new(inner)))
            })
        })
    }

    fn accept_vector_literal(&mut self) -> Option<Result<ExprAst<'a>>> {
        self.try_spanned(|this| {
            this.accept("[").then(|| {
                let vector_elems = this.parse_expr_list()?;
                this.expect("]")?;
                Ok(ExprAstNode::Vector(vector_elems))
            })
        })
    }

    fn accept_numeric_literal(&mut self) -> Option<Result<ExprAst<'a>>> {
        self.try_spanned(|this| {
            let mut s = this.accept_regex(&GENEROUS_FLOAT_REGEX)?;

            // Handle "pi" suffix on numeric literal.
            if let Some(without_pi) = s.strip_suffix("pi") {
                s = without_pi;
            }

            // Parse numeric literal.
            Some(match s.parse() {
                Ok(n) => Ok(ExprAstNode::Number(n)),
                Err(e) => Err(anyhow!("bad numeric literal {s:?}: {e}")),
            })
        })
    }

    fn accept_func_call(&mut self) -> Option<Result<ExprAst<'a>>> {
        self.try_spanned(|this| {
            let func_name = this.accept_regex(&IDENTIFIER_REGEX)?;

            this.accept("(")
                .then(|| Ok(ExprAstNode::FuncCall(func_name, this.parse_expr_list()?)))
        })
    }

    fn accept_identifier(&mut self) -> Option<Result<ExprAst<'a>>> {
        self.try_spanned(|this| {
            let identifier = this.accept_regex(&IDENTIFIER_REGEX)?;
            Some(Ok(ExprAstNode::Identifier(identifier)))
        })
    }

    fn try_spanned(
        &mut self,
        f: impl FnOnce(&mut Self) -> Option<Result<ExprAstNode<'a>>>,
    ) -> Option<Result<ExprAst<'a>>> {
        self.skip_whitespace();
        let start = self.cursor;
        match f(self) {
            // Parsing was successful; record the span.
            Some(Ok(node)) => {
                let span = self.span_from(start);
                Some(Ok(ExprAst { span, node }))
            }
            // Parsing failed unrecoverably; return the error.
            Some(Err(e)) => Some(Err(e)),
            // Parsing failed, but we can recover; rewind the cursor.
            None => {
                self.cursor = start;
                None
            }
        }
    }
    fn spanned(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<ExprAstNode<'a>>,
    ) -> Result<ExprAst<'a>> {
        self.skip_whitespace();
        let start = self.cursor;
        let node = f(self)?;
        let span = self.span_from(start);
        Ok(ExprAst { span, node })
    }

    fn accept_regex(&mut self, pat: &Regex) -> Option<&'a str> {
        self.skip_whitespace();
        let m = pat.find(self.rest())?;
        self.cursor += m.end();
        Some(m.as_str())
    }

    /// Consumes a symbol. Returns an error if the symbol could not be consumed.
    fn expect(&mut self, symbol: &str) -> Result<()> {
        if self.accept(symbol) {
            Ok(())
        } else {
            Err(anyhow!("expected {symbol:?} at start of {:?}", self.rest()))
        }
    }

    /// Returns whether the next symbol matches.
    #[must_use]
    fn peek(&self, symbol: &str) -> bool {
        self.clone().accept(symbol)
    }

    /// Consumes a symbol if possible. Returns whether the symbol was consumed.
    #[must_use]
    fn accept(&mut self, symbol: &str) -> bool {
        self.skip_whitespace();
        if self.rest().starts_with(symbol) {
            // If `symbol` ends with an alphanumeric character, make sure the
            // next character of the input is not alphanumeric.
            if symbol.ends_with(|c: char| c.is_alphanumeric()) {
                if let Some(c) = self.rest().chars().next() {
                    if c.is_alphanumeric() {
                        return false;
                    }
                }
            }

            self.cursor += symbol.len();
            true
        } else {
            false
        }
    }

    /// Consumes the first of many symbols that matches and returns its
    /// associated value, or `None` if no symbol matches.
    #[must_use]
    fn accept_any<'b, T>(&mut self, mut options: impl Iterator<Item = (&'b str, T)>) -> Option<T> {
        options.find_map(|(s, value)| self.accept(s).then_some(value))
    }

    fn skip_whitespace(&mut self) {
        let after_whitespace = self.rest().trim_start();
        self.cursor += self.rest().len() - after_whitespace.len();
    }
}
