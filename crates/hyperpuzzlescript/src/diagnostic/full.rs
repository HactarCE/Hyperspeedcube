use ariadne::Fmt;

use super::{Diagnostic, Error, ReportBuilder, TracebackLine};
use crate::{FileId, Span, Value};

/// [`Diagnostic`] with primary span and traceback.
#[derive(Debug, Clone)]
pub struct FullDiagnostic {
    /// Error message.
    pub msg: Diagnostic,
    /// Primary span.
    ///
    /// `msg` may contain more spans.
    pub span: Span,
    /// Caller spans.
    pub traceback: Vec<TracebackLine>,
}

impl FullDiagnostic {
    /// Adds a caller span to the error message.
    pub(crate) fn at_caller(mut self, traceback_line: TracebackLine) -> FullDiagnostic {
        if self.span == crate::BUILTIN_SPAN {
            self.span = traceback_line.call_span;
        } else {
            self.traceback.push(traceback_line);
        }
        self
    }

    /// Resolves [`Error::Return`] into a value and converts [`Error::Break`] &
    /// [`Error::Continue`] into errors.
    pub(crate) fn try_resolve_return_value(mut self) -> Result<Value, Self> {
        match &mut self.msg {
            Diagnostic::Error(Error::Return(value)) => Ok(std::mem::take(value)),
            Diagnostic::Error(Error::Break) => Err(Error::BreakOutsideLoop.at(self.span)),
            Diagnostic::Error(Error::Continue) => Err(Error::ContinueOutsideLoop.at(self.span)),
            _ => Err(self),
        }
    }

    /// Resolves [`Error::Break`], and [`Error::Continue`] into control flow.
    pub(crate) fn try_resolve_loop_control_flow(mut self) -> Result<LoopControlFlow, Self> {
        match &mut self.msg {
            Diagnostic::Error(Error::Break) => Ok(LoopControlFlow::Break),
            Diagnostic::Error(Error::Continue) => Ok(LoopControlFlow::Continue),
            _ => Err(self),
        }
    }

    /// Returns the error as a string with ANSI escape codes.
    pub fn to_string(&self, mut files: impl ariadne::Cache<FileId>) -> String {
        match &self.msg {
            Diagnostic::Error(error) => error.report(ReportBuilder::new(
                ariadne::ReportKind::Error,
                error,
                self.span,
            )),
            Diagnostic::Warning(warning) => warning.report(ReportBuilder::new(
                ariadne::ReportKind::Warning,
                warning,
                self.span,
            )),
        }
        .labels(
            // only report first caller span
            self.traceback
                .first()
                .filter(|line| line.call_span != self.span)
                .map(|line| (line.call_span, "in this function call")),
        )
        .notes((!self.traceback.is_empty()).then(|| {
            let mut s = "here is the traceback:"
                .fg(ariadne::Color::Fixed(231))
                .to_string();
            for (i, line) in self.traceback.iter().enumerate() {
                line.write(&mut s, &mut files, i == 0, i + 1 == self.traceback.len());
            }
            s
        }))
        .into_string_with_ansi_escapes(files)
    }
}

#[derive(Debug, Clone)]
pub(crate) enum LoopControlFlow {
    Break,
    Continue,
}
