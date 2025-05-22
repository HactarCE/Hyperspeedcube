use chumsky::prelude::*;
use itertools::Itertools;

use crate::{DiagMsg, FileId, FullDiagnostic, Span, ast};

mod lexer;
mod parser;

pub(crate) use lexer::{CHARS_THAT_MUST_BE_ESCAPED_IN_STRING_LITERALS, LexError};
use lexer::{LexExtra, Token};
pub(crate) use parser::ParseError;
use parser::{ParseExtra, ParserInput};

pub fn parse(file_id: FileId, file_contents: &str) -> Result<ast::Node, Vec<FullDiagnostic>> {
    let full_span = Span {
        start: 0,
        end: file_contents.len() as u32,
        context: file_id,
    };

    // Build lexer.
    let lexer: Boxed<'_, '_, &'_ str, Vec<(Token, Span)>, LexExtra<'_>> = lexer::lexer().boxed();

    // Lex the input.
    let tokens = lexer
        .parse_with_state(file_contents, &mut extra::SimpleState(file_id))
        .into_result()
        .map_err(|errs| {
            errs.into_iter()
                .map(|e| {
                    let span = Span {
                        start: e.span().start as u32,
                        end: e.span().end as u32,
                        context: file_id,
                    };
                    DiagMsg::LexError(e.into_owned()).at(span)
                })
                .collect_vec()
        })?;

    // Build parser.
    let parser: Boxed<'_, '_, ParserInput<'_>, ast::Node, ParseExtra<'_>> =
        parser::parser().boxed();

    // Parse the input.
    parser
        .parse_with_state(
            parser::make_input(full_span, &tokens),
            &mut extra::SimpleState(file_contents),
        )
        .into_result()
        .map_err(|errs| {
            errs.into_iter()
                .map(|e| {
                    let span = *e.span();
                    DiagMsg::ParseError(e.into_owned()).at(span)
                })
                .collect_vec()
        })
}
