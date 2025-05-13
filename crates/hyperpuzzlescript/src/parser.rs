// use ariadne::{Color, Label, Report, ReportKind, sources};
use chumsky::{
    input::BorrowInput,
    pratt::{left, right},
    prelude::*,
};
use itertools::Itertools;

use crate::{
    Span, Spanned, ast,
    lexer::{LexError, StringSegmentToken, Token},
};

pub type ParseError<'src> = Rich<'src, Token<'src>>;

type Extra<'src> = extra::Full<ParseError<'src>, (), Ctx>;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
struct Ctx {
    allow_newlines_within_statement: bool,
}
impl Ctx {
    const DELIMITED: Self = Self {
        allow_newlines_within_statement: true,
    };
}

pub fn parse<'src>(s: &'src str) -> Result<ast::Node, Vec<String>> {
    // TODO: better error handling
    let lexer = crate::lexer::lexer();
    let tokens = lexer
        .parse(s)
        .into_result()
        .map_err(|e| e.into_iter().map(|e| e.to_string()).collect_vec())?;
    let parser = parser(make_input);
    let ast = parser
        .parse(make_input((0..s.len()).into(), &tokens))
        .into_result()
        .map_err(|e| {
            e.into_iter()
                .map(|e| format!("parse error at {}: {}", e.span(), e.reason()))
                .collect_vec()
        })?;
    Ok(ast)
}

#[test]
fn test_things() {
    let p = parser(make_input);
    // dbg!(p.parse(" abc "));
    // dbg!(p.parse(" abc\n "));
    // dbg!(p.parse("[ abc, def ]"));
    // dbg!(p.parse("[\n abc, \ndef\n]"));
    // dbg!(p.parse(r##""my string $("abc $(def) \n $("hij")")""##));
}

fn make_input<'src>(
    eoi: Span,
    toks: &'src [Spanned<Token<'src>>],
) -> impl BorrowInput<'src, Token = Token<'src>, Span = Span> {
    toks.map(eoi, |(t, s)| (t, s))
}

fn optional<'src, I, T>(
    p: impl Clone + Parser<'src, I, T, Extra<'src>>,
) -> impl Clone + Parser<'src, I, Option<T>, Extra<'src>>
where
    I: BorrowInput<'src, Token = Token<'src>, Span = Span>,
{
    p.repeated()
        .at_most(1)
        .collect::<Vec<T>>()
        .map(|xs| xs.into_iter().next())
}

fn parser<'src, I, M>(make_input: M) -> impl Parser<'src, I, ast::Node, Extra<'src>>
where
    I: BorrowInput<'src, Token = Token<'src>, Span = Span>,
    // Because this function is generic over the input type, we need the caller to tell us how to create a new input,
    // `I`, from a nested token tree. This function serves that purpose.
    M: Copy + Fn(Span, &'src [Spanned<Token<'src>>]) -> I + Clone + 'src,
{
    let mut expr = Recursive::declare();
    let mut statement = Recursive::declare();

    let newlines = just(Token::Newline).repeated();

    let parens = select_ref! { Token::Parens(toks) = e => make_input(e.span(), toks) };
    let brackets = select_ref! { Token::Brackets(toks) = e => make_input(e.span(), toks) };
    let braces = select_ref! { Token::Braces(toks) = e => make_input(e.span(), toks) };

    let statement_list = statement
        .clone()
        .separated_by(newlines.clone().at_least(1))
        .collect()
        .map(ast::NodeContents::Block)
        .with_ctx(Ctx::default())
        .padded_by(newlines.clone());

    let statement_block = statement_list.clone().nested_in(braces);
    let spanned_statement_block = statement_block
        .clone()
        .map_with(|block, e| Box::new((block, e.span())));

    // Workaround for https://github.com/zesterer/chumsky/issues/748
    let expr_pad = (newlines.clone()).configure(|repeat, ctx: &Ctx| {
        if ctx.allow_newlines_within_statement {
            repeat
        } else {
            repeat.exactly(0)
        }
    });
    let pad_just = |token: Token<'src>| just(token).padded_by(expr_pad.clone());

    let type_annotation = optional(
        pad_just(Token::Colon)
            .ignore_then(expr.clone())
            .map(Box::new),
    );

    let fn_params = pad_just(Token::Ident)
        .to_span()
        .then(type_annotation.clone())
        .map(|(name, ty)| ast::FnParam { name, ty })
        .separated_by(pad_just(Token::Comma))
        .allow_trailing()
        .collect()
        .with_ctx(Ctx::DELIMITED)
        .nested_in(parens);

    let fn_contents = fn_params
        .clone()
        .then(optional(
            just(Token::Arrow).ignore_then(expr.clone().map(Box::new)),
        ))
        .then(spanned_statement_block.clone())
        .map(|((params, return_type), body)| ast::FnContents {
            params,
            return_type,
            body,
        });

    expr.define({
        let expr_list = expr
            .clone()
            .separated_by(pad_just(Token::Comma))
            .allow_trailing()
            .collect()
            .with_ctx(Ctx::DELIMITED);

        let ident = pad_just(Token::Ident)
            .to_span()
            .map(ast::NodeContents::Ident);

        let expr_clone = expr.clone();
        let string_literal = move |contents: &'src [Spanned<StringSegmentToken<'src>>]| {
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
                                .parse(make_input(*span, items))
                                .into_result()
                                .map_err(|e| e.into_iter().next().expect("empty errors"))?,
                        ))
                    }
                })
                .try_collect()
                .map(ast::NodeContents::StringLiteral)
        };

        let literal = select_ref! {
            Token::Null => Ok(ast::NodeContents::NullLiteral),
            Token::True => Ok(ast::NodeContents::BoolLiteral(true)),
            Token::False => Ok(ast::NodeContents::BoolLiteral(false)),
            Token::NumberLiteral(s) = e => match s.parse() {
                Ok(n) => Ok(ast::NodeContents::NumberLiteral(n)),
                Err(err) => Err(Rich::custom(e.span(), format!("invalid number literal: {err}"))),
            },
            Token::StringLiteral(contents) => string_literal(contents),
        }
        .try_map(|result, _span| result);

        let fn_expr = just(Token::Fn)
            .ignore_then(fn_contents.clone())
            .map(ast::NodeContents::Fn);

        let list_literal = expr_list
            .clone()
            .nested_in(brackets)
            .map(ast::NodeContents::ListLiteral);

        let map_literal = just(Token::Hash).ignore_then(
            (ident.clone().map_with(|x, e| (x, e.span())))
                .then_ignore(just(Token::Colon))
                .then(expr.clone())
                .separated_by(just(Token::Comma))
                .allow_trailing()
                .collect()
                .map(ast::NodeContents::MapLiteral)
                .with_ctx(Ctx::DELIMITED)
                .nested_in(braces),
        );

        let if_else_expr = just(Token::If)
            .ignore_then(expr.clone().map(Box::new))
            .then(
                expr.clone()
                    .map(Box::new)
                    .with_ctx(Ctx::DELIMITED)
                    .nested_in(braces),
            )
            .separated_by(just(Token::Else))
            .at_least(1)
            .collect()
            .then(optional(
                just(Token::Else).ignore_then(
                    expr.clone()
                        .map(Box::new)
                        .with_ctx(Ctx::DELIMITED)
                        .nested_in(braces),
                ),
            ))
            .map(|(if_cases, else_case)| ast::NodeContents::IfElse {
                if_cases,
                else_case,
            });

        let paren_expr = expr
            .clone()
            .with_ctx(Ctx::DELIMITED)
            .nested_in(parens)
            .map(Box::new)
            .map(ast::NodeContents::Paren);

        let atom = choice((
            ident,
            literal,
            fn_expr,
            list_literal,
            map_literal,
            if_else_expr,
            paren_expr,
        ))
        .boxed()
        .labelled("value");

        let op_parser = |s| just(s).to_span().padded_by(expr_pad.clone());
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
            let dot_then_ident = just(Token::Period)
                .ignore_then(just(Token::Ident).to_span())
                .padded_by(expr_pad.clone());
            chumsky::pratt::postfix(binding_power, dot_then_ident, |lhs, field, extra| {
                let obj = Box::new(lhs);
                (ast::NodeContents::Access { obj, field }, extra.span())
            })
        };
        let postfix_function_call = |binding_power| {
            let paren_expr_list = expr_list
                .clone()
                .nested_in(parens)
                .padded_by(expr_pad.clone());
            chumsky::pratt::postfix(binding_power, paren_expr_list, |lhs, args, extra| {
                let func = Box::new(lhs);
                (ast::NodeContents::FnCall { func, args }, extra.span())
            })
        };
        let postfix_indexing = |binding_power| {
            let bracket_expr_list = expr_list
                .clone()
                .nested_in(brackets)
                .padded_by(expr_pad.clone());
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
            .padded_by(expr_pad)
            .boxed()
    });

    statement.define({
        let expr_or_assignment = expr
            .clone()
            .then(optional(
                type_annotation
                    .clone()
                    .then(choice([just(Token::Assign), just(Token::CompoundAssign)]).to_span())
                    .then(expr.clone()),
            ))
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
            .separated_by(just(Token::Comma))
            .at_least(1)
            .collect();
        let ident_list_in_parens = just(Token::Ident)
            .to_span()
            .padded_by(newlines.clone())
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .at_least(1)
            .collect()
            .with_ctx(Ctx::DELIMITED)
            .nested_in(parens);
        let ident_list = choice((bare_ident_list, ident_list_in_parens));

        let fn_declaration = just(Token::Fn)
            .ignore_then(just(Token::Ident).to_span())
            .then(fn_contents.clone().map(Box::new))
            .map(|(name, contents)| ast::NodeContents::FnDef { name, contents });

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
            .then(optional(
                just(Token::Else).ignore_then(spanned_statement_block.clone()),
            ))
            .map(|(if_cases, else_case)| ast::NodeContents::IfElse {
                if_cases,
                else_case,
            });

        let for_loop = just(Token::For)
            .ignore_then(ident_list)
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
            .ignore_then(optional(expr.clone().map(Box::new)))
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
            // Assignment (last because it's the hardest to parse)
            expr_or_assignment,
        ))
        .map_with(|stmt, e| (stmt, e.span()))
        .boxed()
    });

    statement_list.map_with(|block, e| (block, e.span()))
}
