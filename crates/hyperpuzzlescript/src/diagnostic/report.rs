use crate::{FileId, Span};

pub struct ReportBuilder {
    builder: ariadne::ReportBuilder<'static, AriadneSpan>,
    main_span: AriadneSpan,
    next_color: Box<dyn FnMut() -> ariadne::Color>,
}
impl ReportBuilder {
    fn new(
        kind: ariadne::ReportKind<'static>,
        msg: impl ToString,
        code: u32,
        span: impl Into<AriadneSpan>,
    ) -> Self {
        let span = span.into();
        Self {
            builder: ariadne::Report::build(kind, span)
                .with_code(code)
                .with_message(msg),
            main_span: span,
            next_color: Box::new(color_generator()),
        }
    }
    pub fn error(code: u32, msg: impl ToString, span: impl Into<AriadneSpan>) -> Self {
        Self::new(ariadne::ReportKind::Error, msg, code, span)
    }
    pub fn warning(code: u32, msg: impl ToString, span: impl Into<AriadneSpan>) -> Self {
        Self::new(ariadne::ReportKind::Warning, msg, code, span)
    }

    pub fn main_label(self, msg: impl ToString) -> Self {
        let span = self.main_span;
        self.label(span, msg)
    }
    pub fn label(self, span: impl Into<AriadneSpan>, msg: impl ToString) -> Self {
        self.labels([(span, msg)])
    }
    pub fn labels<S: Into<AriadneSpan>, L: ToString>(
        mut self,
        spanned_labels: impl IntoIterator<Item = (S, L)>,
    ) -> Self {
        self.builder.add_labels(
            spanned_labels
                .into_iter()
                .map(|(span, label)| new_label((self.next_color)(), span, label)),
        );
        self
    }

    pub fn note(mut self, note: impl ToString) -> Self {
        self.builder.add_note(note);
        self
    }
    pub fn notes<S: ToString>(mut self, notes: impl IntoIterator<Item = S>) -> Self {
        for note in notes {
            self = self.note(note);
        }
        self
    }
    pub fn help(mut self, help: impl ToString) -> Self {
        self.builder.add_help(help);
        self
    }

    pub fn label_or_note(
        self,
        opt_span: Option<impl Into<AriadneSpan>>,
        msg: impl ToString,
    ) -> Self {
        match opt_span {
            Some(span) => self.label(span, msg),
            None => self.note(msg),
        }
    }

    pub fn to_string_with_ansi_escapes(self, files: impl ariadne::Cache<FileId>) -> String {
        let mut out = vec![];
        match self.builder.finish().write(files, &mut out) {
            Ok(()) => String::from_utf8_lossy(&out).into_owned(),
            Err(e) => format!("internal error: {e}"),
        }
    }
}

fn new_label(
    color: ariadne::Color,
    span: impl Into<AriadneSpan>,
    msg: impl ToString,
) -> ariadne::Label<AriadneSpan> {
    use ariadne::Fmt;

    ariadne::Label::new(span.into())
        .with_message(msg.to_string().fg(color))
        .with_color(color)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct AriadneSpan(Span);
impl ariadne::Span for AriadneSpan {
    type SourceId = FileId;

    fn source(&self) -> &Self::SourceId {
        &self.0.context
    }
    fn start(&self) -> usize {
        self.0.start as usize
    }
    fn end(&self) -> usize {
        self.0.end as usize
    }
}
impl<T: Copy + Into<AriadneSpan>> From<&T> for AriadneSpan {
    fn from(value: &T) -> Self {
        (*value).into()
    }
}
impl From<Span> for AriadneSpan {
    fn from(value: Span) -> Self {
        Self(value)
    }
}
impl From<(FileId, chumsky::span::SimpleSpan)> for AriadneSpan {
    fn from((file_id, span): (FileId, chumsky::span::SimpleSpan)) -> Self {
        Self(Span {
            start: span.start as u32,
            end: span.end as u32,
            context: file_id,
        })
    }
}

fn color_generator() -> impl FnMut() -> ariadne::Color {
    let mut ariadne_color_generator = ariadne::ColorGenerator::new();
    ariadne_color_generator.next();

    let mut iter = itertools::chain(
        // some nice colors selected from https://www.calmar.ws/vim/256-xterm-24bit-rgb-color-chart.html
        [81, 207, 220, 156, 211, 104, 208, 49, 26, 101].map(ariadne::Color::Fixed),
        std::iter::from_fn(move || Some(ariadne_color_generator.next())),
    );

    move || iter.next().unwrap()
}
