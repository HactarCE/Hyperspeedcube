use std::sync::Arc;

use chumsky::pratt::{left, right};
use chumsky::prelude::*;

use super::lexer::{StringSegmentToken, Token};
use crate::{Span, Spanned, ast};

pub(super) type ParserInput<'src> = chumsky::input::MappedInput<
    Token,
    Span,
    &'src [Spanned<Token>],
    fn(&'src Spanned<Token>) -> (&'src Token, &'src Span),
>;
pub(super) type ParseError<'src> = Rich<'src, Token, Span>;
pub(super) type ParseState<'src> = extra::SimpleState<&'src str>;
pub(super) type ParseExtra<'src> = extra::Full<ParseError<'src>, ParseState<'src>, ()>;

/// Adds a label to an error at `span` saying "inside this {thing}".
fn map_err_inside_this<'src>(
    thing: &str,
    span: Span,
) -> impl Copy + Fn(ParseError<'src>) -> ParseError<'src> {
    move |mut e| {
        chumsky::label::LabelError::<ParserInput<'_>, _>::in_context(
            &mut e,
            format!("inside this {thing}"),
            span,
        );
        e
    }
}
/// Adds a label to an error at `span` saying "inside this {thing}".
fn inside_this<'src>(
    thing: &str,
) -> impl Copy + Fn(ParseError<'src>, Span, &mut ParseState<'src>) -> ParseError<'src> {
    move |e, span, _state| map_err_inside_this(thing, span)(e)
}

/// Converts `&[Spanned<Token>]` to [`ParserInput`].
pub fn make_input<'src>(eoi: Span, toks: &'src [Spanned<Token>]) -> ParserInput<'src> {
    toks.map(eoi, |(t, s)| (t, s))
}

/// Returns the string slice at `span`.
fn span_to_str<'src>(span: Span, e: &mut ParseState<'src>) -> &'src str {
    &e[span.start as usize..span.end as usize]
}

pub fn parser<'src>() -> impl Parser<'src, ParserInput<'src>, ast::Node, ParseExtra<'src>> {
    let mut expr = Recursive::declare();
    let mut statement = Recursive::declare();

    let comma_sep = just(Token::Comma).recover_with(skip_then_retry_until(
        any().ignored(),
        just(Token::Comma).ignored(),
    ));

    let parens = select_ref! { Token::Parens(toks) = e => make_input(e.span(), toks) };
    let brackets = select_ref! { Token::Brackets(toks) = e => make_input(e.span(), toks) };
    let braces = select_ref! { Token::Braces(toks) = e => make_input(e.span(), toks) };
    let map_literal = select_ref! { Token::MapLiteral(toks) = e => make_input(e.span(), toks) };

    let statement_list = statement
        .clone()
        .separated_by(just(Token::Newline).repeated().at_least(1))
        .allow_leading()
        .allow_trailing()
        .collect()
        .map(ast::NodeContents::Block)
        .boxed();

    let statement_block = statement_list
        .clone()
        .nested_in(braces)
        .labelled("statement block");
    let spanned_statement_block = statement_block
        .clone()
        .map_with(|block, e| Box::new((block, e.span())))
        .boxed();

    let opt_type_annotation = just(Token::Colon)
        .ignore_then(expr.clone())
        .map(Box::new)
        .or_not()
        .boxed();

    let fn_params = (just(Token::Ident).to_span())
        .then(opt_type_annotation.clone())
        .map(|(name, ty)| ast::FnParam { name, ty })
        .separated_by(comma_sep.clone())
        .allow_trailing()
        .collect()
        .boxed()
        .nested_in(parens);

    let fn_contents = fn_params
        .clone()
        .then(
            just(Token::Arrow)
                .ignore_then(expr.clone().map(Box::new))
                .or_not(),
        )
        .then(spanned_statement_block.clone())
        .map(|((params, return_type), body)| ast::FnContents {
            params,
            return_type,
            body: Arc::new(*body),
        });

    expr.define({
        let expr_list = expr
            .clone()
            .separated_by(comma_sep.clone())
            .allow_trailing()
            .collect();

        let ident = just(Token::Ident).to_span().map(ast::NodeContents::Ident);

        let expr_clone = expr.clone();
        let string_literal_contents =
            move |contents: &'src [Spanned<StringSegmentToken>], state: &mut ParseState<'src>| {
                contents
                    .iter()
                    .map(|(token, span)| match token {
                        StringSegmentToken::Literal => Ok(ast::StringSegment::Literal(*span)),
                        StringSegmentToken::Escape(c) => match c {
                            'n' => Ok(ast::StringSegment::Char('\n')),
                            c if c.is_ascii_punctuation() => Ok(ast::StringSegment::Char(*c)),
                            c => {
                                let msg = format!("unknown escape character: {c:?}");
                                Err(Rich::custom(*span, msg))
                            }
                        },
                        StringSegmentToken::Interpolation(items) => {
                            Ok(ast::StringSegment::Interpolation(
                                expr_clone
                                    .parse_with_state(make_input(*span, items), state)
                                    .into_result()
                                    .map_err(|e| e.into_iter().next().expect("empty errors"))?,
                            ))
                        }
                    })
                    .collect::<Result<_, _>>()
                    .map(ast::NodeContents::StringLiteral)
            };
        let string_literal = select_ref! {
            Token::StringLiteral(contents) = e => string_literal_contents(contents, e.state())
                .map_err(map_err_inside_this("string", e.span())),
        }
        .try_map(|result, _span| result);

        let literal = select_ref! {
            Token::Null => Ok(ast::NodeContents::NullLiteral),
            Token::True => Ok(ast::NodeContents::BoolLiteral(true)),
            Token::False => Ok(ast::NodeContents::BoolLiteral(false)),
            Token::NumberLiteral = e => match span_to_str(e.span(), e.state()).parse() {
                Ok(n) => Ok(ast::NodeContents::NumberLiteral(n)),
                Err(err) => Err(Rich::custom(
                    e.span(),
                    format!("invalid number literal: {err}"),
                )),
            },
        }
        .try_map(|result, _span| result);

        let list_literal = expr_list
            .clone()
            .map_err_with_state(inside_this("list"))
            .nested_in(brackets)
            .map(ast::NodeContents::ListLiteral);

        let map_literal = choice((ident.clone(), string_literal.clone()))
            .map_with(|x, e| (x, e.span()))
            .then_ignore(just(Token::Colon))
            .then(expr.clone())
            .separated_by(comma_sep.clone())
            .allow_trailing()
            .collect()
            .map_err_with_state(inside_this("map"))
            .map(ast::NodeContents::MapLiteral)
            .nested_in(map_literal);

        let if_else_expr = just(Token::If)
            .ignore_then(expr.clone().map(Box::new))
            .then(expr.clone().map(Box::new).nested_in(braces))
            .separated_by(just(Token::Else))
            .at_least(1)
            .collect()
            .then(
                just(Token::Else)
                    .ignore_then(expr.clone().map(Box::new).nested_in(braces))
                    .or_not(),
            )
            .map(|(if_cases, else_case)| ast::NodeContents::IfElse {
                if_cases,
                else_case,
            });

        let fn_expr = just(Token::Fn)
            .ignore_then(fn_contents.clone())
            .map(ast::NodeContents::Fn)
            .map_err_with_state(inside_this("function"));

        let paren_expr = expr
            .clone()
            .nested_in(parens)
            .map(Box::new)
            .map(ast::NodeContents::Paren);

        let atom = choice((
            ident,
            literal,
            string_literal,
            list_literal,
            map_literal,
            if_else_expr,
            fn_expr,
            paren_expr,
        ))
        .boxed()
        .labelled("value");

        let op_parser = |s| just(s).to_span();
        let prefix = |binding_power, op_str| {
            chumsky::pratt::prefix(binding_power, op_parser(op_str), |op, rhs, extra| {
                let args = vec![rhs];
                (ast::NodeContents::Op { op, args }, extra.span())
            })
        };
        let infix = |binding_power, op_str| {
            chumsky::pratt::infix(binding_power, op_parser(op_str), |lhs, op, rhs, extra| {
                let args = vec![lhs, rhs];
                (ast::NodeContents::Op { op, args }, extra.span())
            })
        };

        let postfix_dot_access = |binding_power| {
            let dot_then_ident = just(Token::Period).ignore_then(just(Token::Ident).to_span());
            chumsky::pratt::postfix(binding_power, dot_then_ident, |lhs, field, extra| {
                let obj = Box::new(lhs);
                (ast::NodeContents::Access { obj, field }, extra.span())
            })
        };
        let postfix_function_call = |binding_power| {
            let paren_expr_list = expr_list.clone().nested_in(parens);
            chumsky::pratt::postfix(binding_power, paren_expr_list, |lhs, args, extra| {
                let func = Box::new(lhs);
                (ast::NodeContents::FnCall { func, args }, extra.span())
            })
        };
        let postfix_indexing = |binding_power| {
            let bracket_expr_list = expr_list
                .clone()
                .nested_in(brackets)
                .map_with(|exprs, e| Box::new((exprs, e.span())));
            chumsky::pratt::postfix(binding_power, bracket_expr_list, |lhs, args, extra| {
                let obj = Box::new(lhs);
                (ast::NodeContents::Index { obj, args }, extra.span())
            })
        };

        atom.map_with(|x, e| (x, e.span()))
            .pratt((
                // Postfix operators
                postfix_dot_access(70),
                postfix_function_call(70),
                postfix_indexing(70),
                // Prefix operators
                prefix(60, Token::Plus),
                prefix(60, Token::Minus),
                prefix(60, Token::Bang),
                prefix(60, Token::Tilde), // bitwise/setwise complement
                // Arithmetic
                vec![
                    infix(right(52), Token::DoubleStar),
                    infix(left(51), Token::Star),
                    infix(left(51), Token::Slash),
                    infix(left(51), Token::Percent),
                    infix(left(50), Token::Plus),
                    infix(left(50), Token::Minus),
                ],
                // Bitwise/setwise/GA operators
                vec![
                    infix(left(43), Token::LeftShift),
                    infix(left(43), Token::RightShift),
                    infix(left(43), Token::LeftContract),
                    infix(left(43), Token::RightContract),
                    infix(left(42), Token::Ampersand),
                    infix(left(41), Token::Caret),
                    infix(left(40), Token::Pipe),
                ],
                // Ranges
                infix(left(30), Token::RangeExclusive),
                infix(left(30), Token::RangeInclusive),
                // Null-coalescing
                infix(left(20), Token::DoubleQuestionMark),
                // Comparison
                vec![
                    infix(left(10), Token::In),
                    infix(left(10), Token::Is),
                    infix(left(10), Token::Eql),
                    infix(left(10), Token::Neq),
                    infix(left(10), Token::Lt),
                    infix(left(10), Token::Gt),
                    infix(left(10), Token::Lte),
                    infix(left(10), Token::Gte),
                ],
                // Boolean logic
                prefix(4, Token::Not),
                infix(left(3), Token::And),
                infix(left(2), Token::Xor),
                infix(left(1), Token::Or),
            ))
            .boxed()
    });

    statement.define({
        let expr_or_assignment = expr
            .clone()
            .then(
                choice((
                    just(Token::CompoundAssign)
                        .to_span()
                        .map(|assign_symbol| (None, assign_symbol)),
                    opt_type_annotation
                        .clone()
                        .then(just(Token::Assign).to_span()),
                ))
                .then(expr.clone())
                .or_not(),
            )
            .map(|(lhs, opt_assignment)| match opt_assignment {
                None => lhs.0,
                Some(((ty, assign_symbol), rhs)) => ast::NodeContents::Assign {
                    var: Box::new(lhs),
                    ty,
                    assign_symbol,
                    value: Box::new(rhs),
                },
            });

        let import_path = just(Token::Ident)
            .to_span()
            .separated_by(just(Token::Period))
            .at_least(1)
            .collect()
            .map(ast::ImportPath);

        let bare_ident_list = just(Token::Ident)
            .to_span()
            .separated_by(comma_sep.clone())
            .at_least(1)
            .collect();
        let ident_list_in_parens = just(Token::Ident)
            .to_span()
            .separated_by(comma_sep.clone())
            .allow_trailing()
            .at_least(1)
            .collect()
            .nested_in(parens);
        let ident_list = choice((bare_ident_list, ident_list_in_parens));

        let fn_declaration = just(Token::Fn)
            .ignore_then(just(Token::Ident).to_span())
            .then(fn_contents.clone().map(Box::new))
            .map(|(name, contents)| ast::NodeContents::FnDef { name, contents })
            .map_err_with_state(inside_this("function"));

        let export_statement = just(Token::Export)
            .ignore_then(
                (expr_or_assignment.clone())
                    .or(fn_declaration.clone())
                    .map_with(|node, e| Box::new((node, e.span()))),
            )
            .map(ast::NodeContents::Export);

        let import_statement = just(Token::Import).ignore_then(choice((
            // import * from path.to.source
            just(Token::Star)
                .ignore_then(import_path.clone())
                .map(ast::NodeContents::ImportAllFrom),
            // import path.to.source as name
            (import_path.clone())
                .then_ignore(just(Token::As))
                .then(just(Token::Ident).to_span())
                .map(|(path, name)| ast::NodeContents::ImportAs(path, name)),
            // import a, b, c from path.to.source
            // import (a, b, c) from path.to.source
            (ident_list.clone())
                .then_ignore(just(Token::From))
                .then(import_path.clone())
                .map(|(members, path)| ast::NodeContents::ImportFrom(members, path)),
            // import source
            just(Token::Ident).to_span().map(ast::NodeContents::Import),
        )));

        let use_statement = just(Token::Use).ignore_then(choice((
            // use * from expr
            just(Token::Star)
                .ignore_then(expr.clone().map(Box::new))
                .map(ast::NodeContents::UseAllFrom),
            // use a, b, c from expr
            ident_list
                .clone()
                .then_ignore(just(Token::From))
                .then(expr.clone().map(Box::new))
                .map(|(members, expr)| ast::NodeContents::UseFrom(members, expr)),
        )));

        let if_else_statement = just(Token::If)
            .ignore_then(expr.clone().map(Box::new))
            .then(spanned_statement_block.clone())
            .separated_by(just(Token::Else))
            .at_least(1)
            .collect()
            .then(
                just(Token::Else)
                    .ignore_then(spanned_statement_block.clone())
                    .or_not(),
            )
            .map(|(if_cases, else_case)| ast::NodeContents::IfElse {
                if_cases,
                else_case,
            });

        let for_loop = just(Token::For)
            .ignore_then(ident_list.map_with(|idents, e| Box::new((idents, e.span()))))
            .then_ignore(just(Token::In))
            .then(expr.clone().map(Box::new))
            .then(spanned_statement_block.clone())
            .map(|((loop_vars, iterator), body)| ast::NodeContents::ForLoop {
                loop_vars,
                iterator,
                body,
            });

        let while_loop = just(Token::While)
            .ignore_then(expr.clone().map(Box::new))
            .then(spanned_statement_block.clone())
            .map(|(condition, body)| ast::NodeContents::WhileLoop { condition, body });

        let continue_statement = just(Token::Continue).map(|_| ast::NodeContents::Continue);
        let break_statement = just(Token::Break).map(|_| ast::NodeContents::Break);
        let return_statement = just(Token::Return)
            .ignore_then(expr.clone().map(Box::new).or_not())
            .map(ast::NodeContents::Return);

        choice((
            // Declarations
            export_statement,
            fn_declaration,
            import_statement,
            use_statement,
            // Control flow
            statement_block,
            if_else_statement,
            for_loop,
            while_loop,
            continue_statement,
            break_statement,
            return_statement,
            // Assignment (last because it doesn't start with a keyword)
            expr_or_assignment,
        ))
        .recover_with(skip_until(
            any().ignored(),
            just(Token::Newline).ignored(),
            || ast::NodeContents::Error,
        ))
        .map_with(|stmt, e| (stmt, e.span()))
        .boxed()
    });

    statement_list.map_with(|block, e| (block, e.span()))
}
