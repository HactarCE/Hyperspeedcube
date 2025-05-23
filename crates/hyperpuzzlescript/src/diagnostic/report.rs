use crate::{FileId, Span, Spanned, Type, Value};

pub struct ReportBuilder {
    builder: ariadne::ReportBuilder<'static, AriadneSpan>,
    main_span: AriadneSpan,
    next_color: Box<dyn FnMut() -> ariadne::Color>,
    label_count: i32,
}
impl ReportBuilder {
    pub fn new(
        kind: ariadne::ReportKind<'static>,
        msg: impl ToString,
        span: impl Into<AriadneSpan>,
    ) -> Self {
        let span = span.into();
        Self {
            builder: ariadne::Report::build(kind, span)
                .with_message(msg)
                .with_config(ariadne::Config::new()),
            main_span: span,
            next_color: Box::new(color_generator()),
            label_count: 0,
        }
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
        self.builder
            .add_labels(spanned_labels.into_iter().map(|(span, label)| {
                self.label_count += 1;
                new_label((self.next_color)(), span, label, self.label_count)
            }));
        self
    }

    pub fn label_value(self, value: &Value) -> Self {
        self.label(value.span, value.repr())
    }
    pub fn label_values<'a>(mut self, values: impl IntoIterator<Item = &'a Value>) -> Self {
        for v in values {
            self = self.label_value(v);
        }
        self
    }

    pub fn label_type(self, (ty, span): &Spanned<Type>) -> Self {
        self.label(span, format!("this is a \x02{ty}\x03"))
    }
    pub fn label_types<'a>(mut self, types: impl IntoIterator<Item = &'a Spanned<Type>>) -> Self {
        for ty in types {
            self = self.label_type(ty);
        }
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

    pub fn into_string_with_ansi_escapes(self, files: impl ariadne::Cache<FileId>) -> String {
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
    order: i32,
) -> ariadne::Label<AriadneSpan> {
    use ariadne::Fmt;

    let mut msg = msg.to_string();
    if msg.contains('\x02') {
        while let (Some(i), Some(j)) = (msg.find('\x02'), msg.find('\x03')) {
            msg.replace_range(i..j + 1, &msg[i + 1..j].fg(color).to_string());
        }
    } else {
        msg = msg.fg(color).to_string();
    }

    ariadne::Label::new(span.into())
        .with_message(msg)
        .with_color(color)
        .with_order(order)
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

fn color_generator() -> impl FnMut() -> ariadne::Color {
    let mut ariadne_color_generator = ariadne::ColorGenerator::new();
    ariadne_color_generator.next();

    let mut iter = itertools::chain(
        // some nice colors selected from https://www.calmar.ws/vim/256-xterm-24bit-rgb-color-chart.html
        [81, 207, 220, 156, 211, 104, 208, 49, 26, 101].map(ariadne::Color::Fixed),
        std::iter::from_fn(move || Some(ariadne_color_generator.next())),
    );

    move || iter.next().expect("ran out of colors")
}
