use arcstr::Substr;
use ariadne::Fmt;

use crate::{FileId, Span};

/// Line in a diagnostic traceback.
#[derive(Debug, Clone)]
pub struct TracebackLine {
    /// Function name, or `None` for anonymous functions.
    pub fn_name: Option<Substr>,
    /// Span where the function was defined, or `None` for built-in functions.
    pub fn_span: Option<Span>,
    /// Span where the function was called.
    pub call_span: Span,
}
impl TracebackLine {
    /// Writes a line of the traceback to a string.
    pub(super) fn write(
        &self,
        out: &mut String,
        mut files: impl ariadne::Cache<FileId>,
        is_first: bool,
        is_last: bool,
    ) {
        *out += if is_first { "\n┬\n" } else { "\n│\n" };
        *out += if is_last { "╰─ " } else { "├─ " };
        match &self.fn_name {
            Some(name) => *out += &name.fg(ariadne::Color::Fixed(49)).to_string(),
            None => *out += "<anonymous fn>",
        }
        if let Some(span) = self.fn_span {
            *out += " (function defined at ";
            *out += &display_span(span, &mut files)
                .fg(ariadne::Color::Fixed(229))
                .to_string();
            *out += ")";
        } else {
            *out += " (built-in function)"
        }
        *out += if is_last { "\n   " } else { "\n│  " };
        *out += "  called at ";
        *out += &display_span(self.call_span, &mut files)
            .fg(ariadne::Color::Fixed(140))
            .to_string();
    }
}

fn display_span(span: Span, mut files: impl ariadne::Cache<FileId>) -> String {
    // Display file name
    match files.display(&span.context) {
        Some(name) => {
            // IIFE to mimic try_block
            let location_suffix = (|| {
                let source = files.fetch(&span.context).ok()?;
                let (line, line_idx, col_idx) = source.get_byte_line(span.start as usize)?;
                let line_text = source.get_line_text(line).unwrap();
                let col_char_idx = line_text[..col_idx.min(line_text.len())].chars().count();
                let line_number = line_idx + 1 + source.display_line_offset();
                let column_number = col_char_idx + 1;
                Some(format!(":{line_number}:{column_number}"))
            })()
            .unwrap_or(String::new());

            format!("{name}{location_suffix}")
        }
        None => "<internal>".to_string(),
    }
}
