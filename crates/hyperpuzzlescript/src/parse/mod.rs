use std::fmt;

use chumsky::prelude::*;

use crate::{Error, FileId, FullDiagnostic, Span, ast};

mod lexer;
mod parser;

use lexer::{LexExtra, Token};
use parser::{ParseExtra, ParserInput};

pub(crate) const CHARS_THAT_MUST_BE_ESCAPED_IN_STRING_LITERALS: &str = "\"\\$";

/// Parses a file and returns an AST node if there were no errors, or a list of
/// diagnostics if there were any errors.
pub fn parse(file_id: FileId, file_contents: &str) -> Result<ast::Node, Vec<FullDiagnostic>> {
    let full_span = span_with_file(file_id, 0..file_contents.len());

    // Build lexer.
    let lexer: Boxed<'_, '_, &'_ str, Vec<(Token, Span)>, LexExtra<'_>> = lexer::lexer().boxed();

    // Lex the input.
    let mut lex_state = extra::SimpleState(file_id);
    let tokens = lexer
        .parse_with_state(file_contents, &mut lex_state)
        .into_result()
        .map_err(|errs| errors_from_rich_errors(errs, |&s| span_with_file(file_id, s)))?;

    // Build parser.
    let parser: Boxed<'_, '_, ParserInput<'_>, ast::Node, ParseExtra<'_>> =
        parser::parser().boxed();

    // Parse the input.
    let mut parse_state = extra::SimpleState(file_contents);
    parser
        .parse_with_state(parser::make_input(full_span, &tokens), &mut parse_state)
        .into_result()
        .map_err(|errs| errors_from_rich_errors(errs, |&s| s))
}

fn errors_from_rich_errors<T: fmt::Display, S: fmt::Display>(
    errs: Vec<Rich<'_, T, S>>,
    make_span: impl Fn(&S) -> Span,
) -> Vec<FullDiagnostic> {
    errs.into_iter()
        .map(|e| {
            let reason = e.reason().to_string();
            let contexts = e
                .contexts()
                .map(|(pattern, span)| (pattern.to_string(), make_span(span)))
                .collect();
            Error::SyntaxError { reason, contexts }.at(make_span(e.span()))
        })
        .collect()
}

fn span_with_file(file_id: FileId, simple_span: impl Into<SimpleSpan>) -> Span {
    let simple_span = simple_span.into();
    Span {
        start: simple_span.start as u32,
        end: simple_span.end as u32,
        context: file_id,
    }
}
