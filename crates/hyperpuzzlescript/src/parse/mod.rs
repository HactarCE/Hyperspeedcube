use std::fmt;

use chumsky::prelude::*;

use crate::{Error, FileId, FullDiagnostic, Span, ast};

mod lexer;
mod parser;

pub use lexer::is_valid_ident;
use lexer::{LexExtra, Token};
use parser::{ParseExtra, ParserInput, specialize_error};

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
        .map_err(|errs| {
            errors_from_rich_errors(
                errs.into_iter()
                    .map(|e| e.map_span(|s| span_with_file(file_id, s)))
                    .collect(),
                |e| e,
            )
        })?;

    // Build parser.
    let parser: Boxed<'_, '_, ParserInput<'_>, ast::Node, ParseExtra<'_>> =
        parser::parser().boxed();

    // Parse the input.
    let mut parse_state = extra::SimpleState(file_contents);
    parser
        .parse_with_state(parser::make_input(full_span, &tokens), &mut parse_state)
        .into_result()
        .map_err(|errs| errors_from_rich_errors(errs, specialize_error))
}

fn errors_from_rich_errors<T: fmt::Display>(
    errs: Vec<Rich<'_, T, Span>>,
    map_err: impl Fn(Rich<'_, T, Span>) -> Rich<'_, T, Span>,
) -> Vec<FullDiagnostic> {
    errs.into_iter()
        .map(map_err)
        .map(|e| {
            let reason = e.reason().to_string();
            let contexts = e
                .contexts()
                .map(|(pattern, span)| (pattern.to_string(), *span))
                .collect();
            Error::SyntaxError { reason, contexts }.at(*e.span())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Diagnostic, Error};

    /// Parses `s` and returns the reason and contexts of its single error.
    fn get_parse_err(s: &str) -> (String, Vec<String>) {
        let errs = parse(0, s).unwrap_err();
        assert_eq!(errs.len(), 1, "expected one error for {s:?}: {errs:?}");
        let Diagnostic::Error(Error::SyntaxError { reason, contexts }) = &errs[0].msg else {
            panic!("unexpected diagnostic for {s:?}: {:?}", errs[0].msg);
        };
        let contexts = contexts.iter().map(|(ctx, _)| ctx.clone()).collect();
        (reason.clone(), contexts)
    }

    /// Regression test for https://codeberg.org/zesterer/chumsky/issues/1027
    #[test]
    fn test_deeply_nested_groups() {
        let src = format!("x = {}1{}", "(".repeat(300), ")".repeat(300));
        assert!(parse(0, &src).is_ok());
    }

    #[test]
    fn test_parser_errors() {
        // Errors within a construct are annotated with that construct.
        assert_eq!(get_parse_err(r#"x = "a${ }""#).1, ["inside this string"]);
        assert_eq!(get_parse_err("x = #{ a: #{ b: } }").1, ["inside this map"]);
        assert_eq!(get_parse_err("x = [[1 2]]").1, ["inside this list"]);
        assert_eq!(get_parse_err("x = fn(a) 1").1, ["inside this function"]);
        assert_eq!(get_parse_err(r#"x = "hello"#).1, ["inside here"]);

        // String interpolation errors are recovered and reported at the
        // interpolation rather than as a failure of the whole string token.
        let (reason, _) = get_parse_err(r#"x = "a${ }""#);
        assert!(reason.starts_with("found end of input"), "{reason}");

        // Contexts stack for nested constructs.
        assert_eq!(
            get_parse_err(r#"x = [#{ a: "b${ }" }]"#).1,
            ["inside this string", "inside this map", "inside this list"]
        );
    }
}
