use super::*;

/// Matches a block of statements.
#[derive(Debug, Copy, Clone)]
pub struct Block;
impl_display!(for Block, "code block");
impl SyntaxRule for Block {
    type Output = Vec<ast::Node>;

    fn prefix_matches(&self, mut p: Parser<'_>) -> bool {
        *p.next() == Token::LBrace
    }

    fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        p.parse(Token::LBrace)?;
        std::iter::from_fn(|| {
            parse_one_of!(p, [Statement.map(Some), Token::RBrace.map(|_| None)]).transpose()
        })
        .collect()
    }
}

/// Matches a sequence of statements.
#[derive(Debug, Copy, Clone)]
pub struct BlockContents;
impl_display!(for BlockContents, "statements");
impl SyntaxRule for BlockContents {
    type Output = Vec<ast::Node>;

    fn prefix_matches(&self, mut p: Parser<'_>) -> bool {
        *p.next() != Token::Eof
    }

    fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        std::iter::from_fn(|| p.try_parse(Statement)).collect()
    }
}

/// Matches a statement.
#[derive(Debug, Copy, Clone)]
pub struct Statement;
impl_display!(for Statement, "statement");
impl SyntaxRule for Statement {
    type Output = ast::Node;

    fn prefix_matches(&self, mut p: Parser<'_>) -> bool {
        matches!(
            p.next(),
            Token::Ident
                | Token::LParen
                | Token::If
                | Token::Do
                | Token::While
                | Token::For
                | Token::Continue
                | Token::Break
                | Token::Return
                | Token::Import
                | Token::Export
                | Token::Fn
        )
    }

    fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        parse_one_of!(
            p,
            [
                literal::Ident
                    .map(ast::NodeContents::Ident)
                    .with_span()
                    .and_then(parse_statement_after_expr),
                expr::ParenExpr
                    .with_span()
                    .and_then(parse_statement_after_expr),
                Token::If.and_then(|p, _| {
                    p.prev();
                    p.parse(expr::IfElse)
                }),
                Token::Do.and_then(|p, _| {
                    Err(ParseErrorMsg::Unimplemented("for loops").at(p.span()))
                }),
                Token::While.and_then(|p, _| {
                    let condition = Box::new(p.parse(expr::Expr)?);
                    let body = p.parse(Block)?;
                    Ok(ast::NodeContents::WhileLoop { condition, body })
                }),
                Token::For.and_then(|p, _| {
                    let mut loop_vars = vec![];
                    loop {
                        loop_vars.push(p.parse(literal::Ident)?);
                        let has_another_loop_var = parse_one_of!(
                            p,
                            [Token::Comma.map(|_| true), Token::In.map(|_| false)]
                        )?;
                        if !has_another_loop_var {
                            break;
                        }
                    }

                    let iterator = Box::new(p.parse(expr::Expr)?);
                    let body = p.parse(Block)?;

                    Ok(ast::NodeContents::ForLoop {
                        loop_vars,
                        iterator,
                        body,
                    })
                }),
                Token::Continue.map(|_| ast::NodeContents::Continue),
                Token::Break.map(|_| ast::NodeContents::Break),
                Token::Return.and_then(|p, _| {
                    Ok(ast::NodeContents::Return(Box::new(p.parse(expr::Expr)?)))
                }),
                Token::Import.and_then(|p, _| { todo!("import statement") }),
                Token::Export.and_then(|p, _| { todo!("export statement") }),
                Token::Fn.and_then(|p, _| {
                    let var = p.parse(literal::Ident.map(ast::NodeContents::Ident).with_span())?;
                    let func =
                        p.parse(literal::FnContents.map(ast::NodeContents::Fn).with_span())?;

                    Ok(ast::NodeContents::Assign {
                        var: Box::new(var),
                        ty: None,
                        assign_symbol: None,
                        value: Box::new(func),
                    })
                }),
            ]
        )
    }
}

fn parse_statement_after_expr(
    p: &mut Parser<'_>,
    mut spanned_expr: ast::Node,
) -> ParseResult<ast::NodeContents> {
    let span_start = spanned_expr.span;

    loop {
        let new_expr = match p.peek_next() {
            Token::Colon | Token::Assign | Token::CompoundAssign
                if spanned_expr.inner.is_fn_call() =>
            {
                let ty = match p.try_parse(Token::Colon) {
                    Some(Ok(_)) => Some(Box::new(p.parse(expr::Expr)?)),
                    Some(Err(e)) => return Err(e),
                    None => None,
                };

                parse_one_of!(p, [Token::Assign, Token::CompoundAssign])?;
                let assign_symbol = Some(p.token_substr());

                let value = p.parse(expr::Expr)?;

                return Ok(ast::NodeContents::Assign {
                    var: Box::new(spanned_expr),
                    ty,
                    assign_symbol,
                    value: Box::new(value),
                });
            }
            Token::LBracket | Token::Period | Token::LParen => {
                p.parse(expr::PostfixExpr(spanned_expr))?
            }
            _ if spanned_expr.inner.is_fn_call() => return Ok(spanned_expr.inner),
            _ => break p.expected("assignment operator, `[`, `.`, or `(`"),
        };
        spanned_expr = Spanned {
            span: Span::merge(span_start, p.span()),
            inner: new_expr,
        }
    }
}
