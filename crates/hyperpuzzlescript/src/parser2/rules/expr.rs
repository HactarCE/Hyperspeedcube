use super::*;

/// Matches an expression.
#[derive(Debug, Copy, Clone)]
pub struct Expr;
impl_display!(for Expr, "expression");
impl SyntaxRule for Expr {
    type Output = ast::Node;

    fn prefix_matches(&self, p: Parser<'_>) -> bool {
        ExprBP::default().prefix_matches(p)
    }
    fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        p.parse(ExprBP::default())
    }
}

/// Matches an expression surrounded by parentheses.
#[derive(Debug, Copy, Clone)]
pub struct ParenExpr;
impl_display!(for ParenExpr, "expression surrounded by `(` and `)`");
impl SyntaxRule for ParenExpr {
    type Output = ast::NodeContents;

    fn prefix_matches(&self, mut p: Parser<'_>) -> bool {
        *p.next() == Token::LParen
    }

    fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        p.parse(
            combinators::Surround::paren(Expr)
                .map(Box::new)
                .map(ast::NodeContents::Paren),
        )
    }
}

/// Matches an expression with the given binding power.
#[derive(Debug, Default, Copy, Clone)]
pub struct ExprBP(u8);
impl_display!(for ExprBP, "expression");
impl SyntaxRule for ExprBP {
    type Output = ast::Node;

    fn prefix_matches(&self, mut p: Parser<'_>) -> bool {
        matches!(
            p.next(),
            Token::Ident
                | Token::NumberLiteral
                | Token::StringLiteral(_)
                | Token::LBrace
                | Token::LBracket
                | Token::LParen
                | Token::Null
                | Token::True
                | Token::False
                | Token::If
                | Token::Fn
                | Token::Hash,
        )
    }
    fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        let min_bp = self.0;

        let mut lhs = match prefix_binding_power(p.peek_next()) {
            Some(((), r_bp)) => {
                p.next();
                let op = p.token_substr();
                let op_span = p.span();
                let arg = p.parse(ExprBP(r_bp))?;
                let span = Span::merge(op_span, arg.span);
                let args = vec![arg];
                let inner = ast::NodeContents::Op { op, args };
                Spanned { span, inner }
            }
            None => p.parse(Atom.with_span())?,
        };

        loop {
            if let Some((l_bp, ())) = postfix_binding_power(p.peek_next()) {
                if l_bp < min_bp {
                    break;
                }
                lhs = p.parse(PostfixExpr(lhs).with_span())?;
                continue;
            }

            if let Some((l_bp, r_bp)) = infix_binding_power(p.peek_next()) {
                if l_bp < min_bp {
                    break;
                }

                p.next();
                let op = p.token_substr();
                let rhs = p.parse(ExprBP(r_bp))?;
                let span = Span::merge(lhs.span, rhs.span);
                let args = vec![lhs, rhs];
                let inner = ast::NodeContents::Op { op, args };
                lhs = Spanned { span, inner };
                continue;
            }

            break;
        }

        Ok(lhs)
    }
}

fn prefix_binding_power(token: &Token) -> Option<((), u8)> {
    match token {
        // Operators
        Token::Plus => Some(((), 102)),
        Token::Minus => Some(((), 102)),
        Token::Bang => Some(((), 102)),
        Token::Tilde => Some(((), 102)),

        // Boolean logic
        Token::Not => Some(((), 16)),

        _ => None,
    }
}

fn infix_binding_power(token: &Token) -> Option<(u8, u8)> {
    match token {
        Token::Period => Some((101, 102)),

        // Arithmetic
        Token::DoubleStar => Some((56, 55)), // right-associative
        Token::Star | Token::Slash | Token::Percent => Some((53, 54)),
        Token::Plus | Token::Minus => Some((51, 52)),

        // Bitwise/setwise operators
        Token::RightShift | Token::LeftShift => Some((47, 48)),
        Token::Ampersand => Some((45, 46)),
        Token::Caret => Some((43, 44)),
        Token::Pipe => Some((41, 42)),

        // Type checking
        Token::Is => Some((33, 34)),

        // Null-coalescing
        Token::DoubleQuestionMark => Some((31, 32)),

        // Comparison
        Token::Eql | Token::Neq | Token::Lt | Token::Gt | Token::Lte | Token::Gte => Some((21, 22)),

        // Boolean logic
        Token::And => Some((13, 14)),
        Token::Or => Some((11, 12)),

        // Ranges
        Token::RangeExclusive | Token::RangeInclusive => Some((1, 2)),

        _ => None,
    }
}

fn postfix_binding_power(token: &Token) -> Option<(u8, ())> {
    match token {
        Token::LParen => Some((101, ())),
        Token::LBracket => Some((101, ())),
        Token::Period => Some((101, ())),

        _ => None,
    }
}

/// Matches an index suffix, function call suffix, or property/method access
/// suffix.
#[derive(Debug)]
pub struct PostfixExpr(pub ast::Node);
impl_display!(for PostfixExpr, "`[`, `(`, or `.`");
impl SyntaxRule for PostfixExpr {
    type Output = ast::NodeContents;

    fn prefix_matches(&self, mut p: Parser<'_>) -> bool {
        matches!(p.next(), Token::LBracket | Token::Period | Token::LParen)
    }

    fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        match p.peek_next() {
            Token::LBracket => {
                let index_expr = Box::new(p.parse(combinators::Surround::bracket(expr::Expr))?);
                Ok(ast::NodeContents::Index(Box::new(self.0), index_expr))
            }
            Token::Period => {
                p.next();
                let field = p.parse(literal::Ident)?;
                Ok(ast::NodeContents::Access(Box::new(self.0), field))
            }
            Token::LParen => {
                let args = p.parse(combinators::List {
                    inner: Expr,
                    sep: Token::Comma,
                    start: Token::LParen,
                    end: Token::RParen,
                    allow_trailing_sep: true,
                    allow_empty: false,
                })?;
                Ok(ast::NodeContents::FnCall {
                    func: Box::new(self.0),
                    args,
                })
            }
            _ => p.expected(self),
        }
    }
}

/// Matches an expression atom.
#[derive(Debug, Copy, Clone)]
struct Atom;
impl_display!(for Atom, "atomic expression");
impl SyntaxRule for Atom {
    type Output = ast::NodeContents;

    fn prefix_matches(&self, mut p: Parser<'_>) -> bool {
        matches!(
            p.next(),
            Token::Ident
                | Token::NumberLiteral
                | Token::StringLiteral(_)
                | Token::LBrace
                | Token::LBracket
                | Token::LParen
                | Token::Null
                | Token::True
                | Token::False
                | Token::If
                | Token::Fn
                | Token::Hash,
        )
    }

    fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        parse_one_of!(
            p,
            [
                Token::Null.map(|_| ast::NodeContents::NullLiteral),
                Token::True.map(|_| ast::NodeContents::BoolLiteral(true)),
                Token::False.map(|_| ast::NodeContents::BoolLiteral(false)),
                Token::NumberLiteral.try_map(|span| {
                    let num_parse_result = span.of(p.source).parse();
                    let num = num_parse_result.map_err(|e| ParseErrorMsg::BadNumber(e).at(span))?;
                    Ok(ast::NodeContents::NumberLiteral(num))
                }),
                literal::StringLiteral,
                literal::ListLiteral,
                literal::MapLiteral,
                literal::AnonymousFn,
                literal::Ident.map(ast::NodeContents::Ident),
                IfElse,
                stmt::Block,
                combinators::Surround::paren(Expr),
            ]
        )
    }
}

/// Matches an if-else chain.
#[derive(Debug, Copy, Clone)]
pub struct IfElse;
impl_display!(for IfElse, "if-else expression");
impl SyntaxRule for IfElse {
    type Output = ast::NodeContents;

    fn prefix_matches(&self, mut p: Parser<'_>) -> bool {
        *p.next() == Token::If
    }

    fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        p.parse(Token::If)?;
        let mut if_cases = vec![];
        let mut else_case = None;
        loop {
            let cond = p.parse(expr::Expr)?;
            let block = p.parse(stmt::Block)?;
            if_cases.push((Box::new(cond), block));
            if p.try_parse(Token::Else).is_some() {
                if p.try_parse(Token::If).is_some() {
                    continue;
                } else {
                    else_case = Some(p.parse(stmt::Block)?);
                }
            }
            break;
        }
        Ok(ast::NodeContents::IfElse {
            if_cases,
            else_case,
        })
    }
}

// /// Matches the arguments of a function call, including the surrounding parens.
// #[derive(Debug, Copy, Clone)]
// pub struct FnArgs;
// impl_display!(for FnArgs, "function arguments surrounded by `(` and `)`");
// impl SyntaxRule for FnArgs {
//     type Output = ast::Node;

//     fn prefix_matches(&self, mut p: Parser<'_>) -> bool {
//         *p.next() == Some(Token::LParen)
//     }
//     fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
//         let spanned_args = p.parse(List {
//             inner: TupleExpression,
//             sep: Token::ArgSep,
//             start: Token::FunctionCall,
//             end: Token::RParen,
//             sep_name: "comma",
//             allow_trailing_sep: false,
//             allow_empty: true,
//         })?;
//         let args = spanned_args.inner;

//         Ok(AstNode {
//             span: Span::merge(func.span, spanned_args.span),
//             inner: ast::NodeContents::FunctionCall { func, args },
//         })
//     }
// }

// /// Matches an array literal.
// #[derive(Debug, Copy, Clone)]
// pub struct ArrayLiteral;
// impl_display!(for ArrayLiteral, "array literal such as '{{1, 2; 3, 4}}'");
// impl SyntaxRule for ArrayLiteral {
//     type Output = AstNode;

//     fn prefix_matches(&self, mut p: Parser<'_>) -> bool {
//         p.next() == Some(Token::LBrace)
//     }

//     fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
//         let start_span = p.peek_next_span();

//         let mut rows = vec![vec![]];
//         p.parse(Token::LBrace)?;
//         loop {
//             rows.last_mut().unwrap().push(p.parse(OptionalExpression)?);
//             match p.next() {
//                 Some(Token::ArgSep) => (),                // next cell within row
//                 Some(Token::RowSep) => rows.push(vec![]), // start a new row
//                 Some(Token::RBrace) => break,             // end of array
//                 _ => {
//                     return Err(p.expected_err(crate::util::join_with_conjunction(
//                         "or",
//                         &[
//                             Token::ArgSep.to_string(),
//                             Token::RowSep.to_string(),
//                             Token::RBrace.to_string(),
//                         ],
//                     )));
//                 }
//             }
//         }
//         if rows.last().unwrap().is_empty() && rows.len() > 1 {
//             rows.pop();
//         }

//         let end_span = p.span();

//         if !rows.iter().map(|row| row.len()).all_equal() {
//             return Err(RunErrorMsg::NonRectangularArray.with_span(end_span));
//         }

//         Ok(Spanned {
//             span: Span::merge(start_span, end_span),
//             inner: ast::NodeContents::Array(rows),
//         })
//     }
// }
