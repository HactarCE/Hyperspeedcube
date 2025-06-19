use std::borrow::Cow;
use std::sync::Arc;

use chumsky::pratt::{left, right};
use chumsky::prelude::*;
use itertools::Itertools;

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
pub fn make_input(eoi: Span, toks: &[Spanned<Token>]) -> ParserInput<'_> {
    toks.map(eoi, |(t, s)| (t, s))
}

/// Returns the string slice at `span`.
fn span_to_str<'src>(span: Span, e: &mut ParseState<'src>) -> &'src str {
    &e[span.start as usize..span.end as usize]
}

pub fn parser<'src>() -> impl Parser<'src, ParserInput<'src>, ast::Node, ParseExtra<'src>> {
    let mut expr = Recursive::declare();
    let mut statement = Recursive::declare();

    let boxed_expr = expr.clone().map(Box::new);
    let ident = just(Token::Ident).to_span();

    let comma_sep = just(Token::Comma).recover_with(skip_then_retry_until(
        any().ignored(),
        one_of([Token::Comma, Token::Newline]).ignored(),
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
        .ignore_then(boxed_expr.clone())
        .or_not()
        .boxed();

    let fn_params = choice((
        ident
            .clone()
            .then(opt_type_annotation.clone())
            .then(just(Token::Assign).ignore_then(boxed_expr.clone()).or_not())
            .map(|((name, ty), default)| (ast::FnParam::Param { name, ty, default })),
        just(Token::Star).to_span().map(ast::FnParam::SeqEnd),
        just(Token::DoubleStar)
            .ignore_then(ident.clone())
            .map(ast::FnParam::NamedSplat),
    ))
    .separated_by(comma_sep.clone())
    .allow_trailing()
    .collect()
    .boxed()
    .nested_in(parens);

    let fn_contents = fn_params
        .clone()
        .then(just(Token::Arrow).ignore_then(boxed_expr.clone()).or_not())
        .then(spanned_statement_block.clone())
        .map(|((params, return_type), body)| ast::FnContents {
            params,
            return_type,
            body: Arc::new(*body),
        });

    let special_ident = just(Token::SpecialIdent).try_map_with(|_, e| {
        span_to_str(e.span(), e.state())
            .parse::<ast::SpecialVar>()
            .map_err(|()| Rich::custom(e.span(), "invalid special identifier"))
    });

    expr.define({
        let expr_list = expr
            .clone()
            .separated_by(comma_sep.clone())
            .allow_trailing()
            .collect();

        let ident_expr = ident.clone().map(ast::NodeContents::Ident);

        let special_ident_expr = special_ident.clone().map(ast::NodeContents::SpecialIdent);

        let expr_clone = expr.clone();
        let string_literal_contents =
            move |contents: &'src [Spanned<StringSegmentToken>], state: &mut ParseState<'src>| {
                contents
                    .iter()
                    .map(|&(ref token, span)| {
                        let segment = match token {
                            StringSegmentToken::Literal => ast::StringSegment::Literal,
                            StringSegmentToken::Escape(c) => ast::StringSegment::Char(*c),
                            StringSegmentToken::Interpolation(items) => expr_clone
                                .parse_with_state(make_input(span, items), state)
                                .into_result()
                                .map_err(|e| e.into_iter().next().expect("empty errors"))
                                .map(ast::StringSegment::Interpolation)?,
                        };
                        Ok((segment, span))
                    })
                    .map(|a| if a.is_err() { dbg!(a) } else { a })
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
            Token::FilePath = e => Ok(ast::NodeContents::FilePath(e.span())),
        }
        .try_map(|result, _span| result);

        let list_literal = expr_list
            .clone()
            .map_err_with_state(inside_this("list"))
            .nested_in(brackets)
            .map(ast::NodeContents::ListLiteral);

        let map_literal = choice((
            just(Token::DoubleStar)
                .ignore_then(expr.clone())
                .map_with(|values, e| (values, e.span()))
                .map(|(values, span)| ast::MapEntry::Splat { span, values }),
            choice((ident_expr.clone(), string_literal.clone()))
                .map_with(|x, e| (x, e.span()))
                .then(opt_type_annotation.clone())
                .then(just(Token::Assign).ignore_then(boxed_expr.clone()).or_not())
                .map(|((key, ty), value)| (ast::MapEntry::KeyValue { key, ty, value })),
        ))
        .separated_by(comma_sep.clone())
        .allow_trailing()
        .collect()
        .map_err_with_state(inside_this("map"))
        .map(ast::NodeContents::MapLiteral)
        .nested_in(map_literal);

        let if_else_expr = just(Token::If)
            .ignore_then(boxed_expr.clone())
            .then(boxed_expr.clone().nested_in(braces))
            .separated_by(just(Token::Else))
            .at_least(1)
            .collect()
            .then(
                just(Token::Else)
                    .ignore_then(boxed_expr.clone().nested_in(braces))
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

        let paren_expr = boxed_expr
            .clone()
            .nested_in(parens)
            .map(ast::NodeContents::Paren);

        let atom = choice((
            ident_expr,
            special_ident_expr,
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
        let postfix = |binding_power, op_str| {
            chumsky::pratt::postfix(binding_power, op_parser(op_str), |lhs, op, extra| {
                let args = vec![lhs];
                (ast::NodeContents::Op { op, args }, extra.span())
            })
        };

        let postfix_dot_access = |binding_power| {
            let dot_then_ident = just(Token::Period).ignore_then(ident.clone());
            chumsky::pratt::postfix(binding_power, dot_then_ident, |lhs, field, extra| {
                let obj = Box::new(lhs);
                (ast::NodeContents::Access { obj, field }, extra.span())
            })
        };
        let postfix_function_call = |binding_power| {
            let fn_arg = (ident.clone().then_ignore(just(Token::Assign)).or_not())
                .then(boxed_expr.clone())
                .map(|(name, value)| ast::FnArg { name, value });
            let paren_fn_arg_list = fn_arg
                .separated_by(comma_sep.clone())
                .allow_trailing()
                .collect()
                .nested_in(parens);
            chumsky::pratt::postfix(binding_power, paren_fn_arg_list, |lhs, args, extra| {
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
                postfix(70, Token::Degrees),
                postfix(70, Token::QuestionMark),
                // Prefix operators
                prefix(60, Token::Plus),
                prefix(60, Token::Minus),
                prefix(60, Token::Bang),
                prefix(60, Token::Tilde), // bitwise/setwise complement
                prefix(60, Token::Sqrt),
                prefix(60, Token::Star),
                prefix(60, Token::DoubleStar),
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
                // Concatenation
                infix(left(25), Token::DoublePlus),
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

        let name_as_alias = ident
            .clone()
            .then(just(Token::As).ignore_then(ident.clone()).or_not())
            .map(|(target, alias)| ast::IdentAs { target, alias });

        let bare_name_as_alias_list = name_as_alias
            .clone()
            .separated_by(comma_sep.clone())
            .at_least(1);
        let name_as_alias_list = choice((
            bare_name_as_alias_list.clone().collect(),
            bare_name_as_alias_list
                .allow_trailing()
                .collect()
                .nested_in(parens),
        ));
        let fn_declaration_contents = just(Token::Fn)
            .ignore_then(ident.clone())
            .then(fn_contents.clone().map(Box::new))
            .map_err_with_state(inside_this("function"));
        let fn_declaration = fn_declaration_contents
            .clone()
            .map(|(name, contents)| ast::NodeContents::FnDef { name, contents });

        let export_statement = just(Token::Export).ignore_then(choice((
            // export fn f(...) { ... }
            fn_declaration_contents
                .map(|(name, contents)| ast::NodeContents::ExportFnDef { name, contents }),
            // export a = expr
            // export a: Type = expr
            ident
                .clone()
                .then(opt_type_annotation)
                .then_ignore(just(Token::Assign))
                .then(boxed_expr.clone())
                .map(|((name, ty), value)| ast::NodeContents::ExportAssign { name, ty, value }),
            // export * from expr
            just(Token::Star)
                .then_ignore(just(Token::From))
                .ignore_then(boxed_expr.clone())
                .map(ast::NodeContents::ExportAllFrom),
            // export a, b as c from expr
            // export (a, b as c) from expr
            name_as_alias_list
                .clone()
                .then_ignore(just(Token::From))
                .then(boxed_expr.clone())
                .map(|(members, expr)| ast::NodeContents::ExportFrom(members, expr)),
            // export a
            // export a as b
            name_as_alias.map(ast::NodeContents::ExportAs),
        )));

        let use_statement = just(Token::Use).ignore_then(choice((
            // use * from expr
            just(Token::Star)
                .then_ignore(just(Token::From))
                .ignore_then(boxed_expr.clone())
                .map(ast::NodeContents::UseAllFrom),
            // use a, b as c from expr
            // use (a, b as c) from expr
            name_as_alias_list
                .clone()
                .then_ignore(just(Token::From))
                .then(boxed_expr.clone())
                .map(|(members, expr)| ast::NodeContents::UseFrom(members, expr)),
        )));

        let if_else_statement = just(Token::If)
            .ignore_then(boxed_expr.clone())
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

        let ident_list = ident
            .clone()
            .separated_by(just(Token::Comma))
            .at_least(1)
            .collect();
        let for_loop = just(Token::For)
            .ignore_then(ident_list.map_with(|idents, e| Box::new((idents, e.span()))))
            .then_ignore(just(Token::In))
            .then(boxed_expr.clone())
            .then(spanned_statement_block.clone())
            .map(|((loop_vars, iterator), body)| ast::NodeContents::ForLoop {
                loop_vars,
                iterator,
                body,
            });

        let while_loop = just(Token::While)
            .ignore_then(boxed_expr.clone())
            .then(spanned_statement_block.clone())
            .map(|(condition, body)| ast::NodeContents::WhileLoop { condition, body });

        let continue_statement = just(Token::Continue).map(|_| ast::NodeContents::Continue);
        let break_statement = just(Token::Break).map(|_| ast::NodeContents::Break);
        let return_statement = just(Token::Return)
            .ignore_then(boxed_expr.clone().or_not())
            .map(ast::NodeContents::Return);

        let with_statement = just(Token::With)
            .ignore_then(special_ident)
            .then_ignore(just(Token::Assign))
            .then(boxed_expr.clone())
            .then(spanned_statement_block.clone())
            .map(|((ident, expr), body)| ast::NodeContents::With(ident, expr, body));

        choice((
            // Declarations
            fn_declaration,
            export_statement,
            use_statement,
            // Control flow
            statement_block,
            if_else_statement,
            for_loop,
            while_loop,
            continue_statement,
            break_statement,
            return_statement,
            with_statement,
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

pub fn specialize_error(e: ParseError<'_>) -> ParseError<'_> {
    if matches!(e.found(), Some(Token::Braces(_))) {
        if e.expected()
            .contains(&chumsky::error::RichPattern::Label(Cow::Borrowed("value")))
        {
            return ParseError::custom(*e.span(), "missing `#` on map literal");
        }
    }

    e
}
