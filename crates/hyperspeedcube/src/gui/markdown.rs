use std::borrow::Cow;
use std::sync::Arc;

// - egui heading is 18.0
// - egui body text is 12.5
const HEADING_SIZES: [f32; 6] = [20.0, 16.0, 15.0, 14.0, 13.5, 13.0];

const INDENT_WIDTH: f32 = 6.0;

pub fn md_bold_user_text(s: &str) -> String {
    format!("**{}**", md_escape(s))
}
pub fn md_escape(s: &str) -> Cow<'_, str> {
    if s.chars().any(needs_escape) {
        let mut ret = String::new();
        for c in s.chars() {
            if needs_escape(c) {
                ret.push('\\');
            }
            ret.push(c);
        }
        ret.into()
    } else {
        s.into()
    }
}
fn needs_escape(c: char) -> bool {
    c.is_ascii_punctuation()
}

/// Renders inline Markdown to an `egui::text::LayoutJob`.
#[must_use]
pub fn md_inline(ui: &egui::Ui, markdown: impl AsRef<str>) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    append_md_inline(ui, &mut job, markdown);
    job
}

pub fn append_md_inline(ui: &egui::Ui, job: &mut egui::text::LayoutJob, markdown: impl AsRef<str>) {
    let style = Arc::clone(ui.style());
    let arena = comrak::Arena::new();
    let ast = comrak::parse_document(&arena, markdown.as_ref(), &options());
    for child in ast.children() {
        render_inline_with_job(job, InlineFormatState::new_paragraph(ui, &style), child);
    }
}

pub fn md(ui: &mut egui::Ui, markdown: impl AsRef<str>) -> egui::Response {
    let arena = comrak::Arena::new();
    let ast = comrak::parse_document(&arena, markdown.as_ref(), &options());
    ui.scope(|ui| {
        ui.visuals_mut().indent_has_left_vline = false;
        ui.spacing_mut().indent = INDENT_WIDTH;
        render_block(ui, ast);
    })
    .response
}

fn options() -> comrak::Options<'static> {
    let mut options = comrak::Options::default();
    options.extension.strikethrough = true;
    options.extension.superscript = true;
    options.extension.underline = true;
    options
}

#[derive(Copy, Clone)]
struct InlineFormatState<'a> {
    text_color: egui::Color32,
    strong_text_color: egui::Color32,
    style: &'a egui::Style,

    bold: bool,
    italics: bool,
    underline: bool,
    strikethrough: bool,
    code: bool,
    superscript: bool,
    subscript: bool,
    size: Option<f32>,
}
impl<'a> InlineFormatState<'a> {
    pub fn new_paragraph(ui: &egui::Ui, style: &'a egui::Style) -> Self {
        Self {
            text_color: ui.visuals().text_color(),
            strong_text_color: ui.visuals().strong_text_color(),
            style,

            bold: false,
            italics: false,
            underline: false,
            strikethrough: false,
            code: false,
            superscript: false,
            subscript: false,
            size: None,
        }
    }

    pub fn new_heading(ui: &egui::Ui, style: &'a egui::Style, level: u8) -> Self {
        let i = usize::from(level)
            .saturating_sub(1)
            .clamp(0, HEADING_SIZES.len() - 1);

        Self {
            size: Some(HEADING_SIZES[i]),
            bold: true,
            ..Self::new_paragraph(ui, style)
        }
    }

    pub fn text_format(self) -> egui::TextFormat {
        let style = if self.code {
            egui::TextStyle::Monospace
        } else if self.superscript || self.subscript {
            egui::TextStyle::Small
        } else {
            egui::TextStyle::Body
        };

        let mut font_id = style.resolve(self.style);
        let color = match self.bold {
            true => self.strong_text_color,
            false => self.text_color,
        };
        if let Some(size) = self.size {
            font_id.size = size;
        }

        egui::TextFormat {
            font_id,
            color,
            italics: self.italics,
            underline: match self.underline {
                true => egui::Stroke::new(1.0, color),
                false => egui::Stroke::NONE,
            },
            strikethrough: match self.strikethrough {
                true => egui::Stroke::new(1.0, color),
                false => egui::Stroke::NONE,
            },
            valign: match (self.superscript, self.subscript) {
                (true, false) => egui::Align::TOP,
                (false, true) => egui::Align::BOTTOM,
                _ => egui::Align::Center,
            },
            ..Default::default()
        }
    }
}

fn render_block_children<'a>(ui: &mut egui::Ui, node: &'a comrak::nodes::AstNode<'a>) {
    let mut is_first = true;
    for child in node.children() {
        if is_first {
            is_first = false;
        } else {
            ui.add_space(ui.spacing().item_spacing.y);
        }
        render_block(ui, child);
    }
}
fn render_block<'a>(ui: &mut egui::Ui, node: &'a comrak::nodes::AstNode<'a>) {
    match &node.data.borrow().value {
        comrak::nodes::NodeValue::Document => render_block_children(ui, node),

        comrak::nodes::NodeValue::FrontMatter(_) => (),

        comrak::nodes::NodeValue::BlockQuote => not_implemented_label(ui, "BlockQuote"), /* not implemented */

        comrak::nodes::NodeValue::List(list) => {
            let id = ui.next_auto_id();
            ui.skip_ahead_auto_ids(1);
            ui.vertical(|ui| {
                ui.indent(id, |ui| {
                    match list.list_type {
                        comrak::nodes::ListType::Bullet => {
                            for list_item in node.children() {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label("•");
                                    ui.scope(|ui| render_block(ui, list_item))
                                });
                            }
                        }
                        comrak::nodes::ListType::Ordered => {
                            let mut i = list.start;
                            for list_item in node.children() {
                                // TODO: align numbered lists properly
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(format!("{i}."));
                                    ui.scope(|ui| render_block(ui, list_item));
                                });
                                i += 1;
                            }
                        }
                    }
                });
            });
        }
        comrak::nodes::NodeValue::Item(_) => {
            render_block_children(ui, node);
        }

        comrak::nodes::NodeValue::DescriptionList
        | comrak::nodes::NodeValue::DescriptionItem(_)
        | comrak::nodes::NodeValue::DescriptionTerm
        | comrak::nodes::NodeValue::DescriptionDetails => {
            not_implemented_label(ui, "DescriptionDetails");
        }

        comrak::nodes::NodeValue::CodeBlock(code_block) => {
            ui.code(&code_block.literal);
        }

        comrak::nodes::NodeValue::HtmlBlock(_) => not_implemented_label(ui, "HtmlBlock"),

        comrak::nodes::NodeValue::Paragraph => {
            let style = Arc::clone(ui.style());
            let format_state = InlineFormatState::new_paragraph(ui, &style);
            render_children_wrapped(ui, format_state, node);
        }

        comrak::nodes::NodeValue::Heading(heading) => {
            if heading.level > 1 {
                ui.add_space(ui.spacing().item_spacing.y * 2.0); // extra spacing above headings
            }
            let style = Arc::clone(ui.style());
            let format_state = InlineFormatState::new_heading(ui, &style, heading.level);
            render_children_wrapped(ui, format_state, node);
        }
        comrak::nodes::NodeValue::ThematicBreak => {
            ui.separator();
        }

        comrak::nodes::NodeValue::FootnoteDefinition(_) => {
            not_implemented_label(ui, "FootnoteDefinition");
        }

        comrak::nodes::NodeValue::Table(_)
        | comrak::nodes::NodeValue::TableRow(_)
        | comrak::nodes::NodeValue::TableCell => not_implemented_label(ui, "Table"),

        comrak::nodes::NodeValue::TaskItem(_) => not_implemented_label(ui, "TaskItem"),

        comrak::nodes::NodeValue::Raw(_) => not_implemented_label(ui, "Raw"),

        comrak::nodes::NodeValue::Alert(_) => not_implemented_label(ui, "Alert"),

        comrak::nodes::NodeValue::Text(_) => (),   // inline
        comrak::nodes::NodeValue::SoftBreak => (), // inline
        comrak::nodes::NodeValue::LineBreak => (), // inline
        comrak::nodes::NodeValue::Code(_) => (),   // inline
        comrak::nodes::NodeValue::HtmlInline(_) => (), // inline
        comrak::nodes::NodeValue::Emph => (),      // inline
        comrak::nodes::NodeValue::Strong => (),    // inline
        comrak::nodes::NodeValue::Strikethrough => (), // inline
        comrak::nodes::NodeValue::Superscript => (), // inline
        comrak::nodes::NodeValue::Link(_) => (),   // inline
        comrak::nodes::NodeValue::Image(_) => (),  // inline
        comrak::nodes::NodeValue::FootnoteReference(_) => (), // inline
        comrak::nodes::NodeValue::Math(_) => (),   // inline
        comrak::nodes::NodeValue::MultilineBlockQuote(_) => {
            not_implemented_label(ui, "MultilineBlockQuote");
        }
        comrak::nodes::NodeValue::Escaped => (), // inline
        comrak::nodes::NodeValue::WikiLink(_) => (), // inline
        comrak::nodes::NodeValue::Underline => (), // inline
        comrak::nodes::NodeValue::Subscript => (), // inline
        comrak::nodes::NodeValue::SpoileredText => (), // inline
        comrak::nodes::NodeValue::EscapedTag(_) => (), // inline
    }
}

fn render_children_wrapped<'a>(
    ui: &mut egui::Ui,
    state: InlineFormatState<'_>,
    node: &'a comrak::nodes::AstNode<'a>,
) {
    if node
        .descendants()
        .any(|child| matches!(child.data.borrow().value, comrak::nodes::NodeValue::Link(_)))
    {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            for child in node.children() {
                render_inline_with_ui(ui, state, child);
            }
        });
    } else {
        let mut job = egui::text::LayoutJob::default();
        render_children_inline_with_job(&mut job, state, node);
        ui.label(job);
    }
}

fn render_inline_with_ui<'a>(
    ui: &mut egui::Ui,
    mut state: InlineFormatState<'_>,
    node: &'a comrak::nodes::AstNode<'a>,
) {
    match &node.data.borrow().value {
        comrak::nodes::NodeValue::Link(link_node) => {
            let mut job = egui::text::LayoutJob::default();
            state.text_color = ui.visuals().hyperlink_color;
            state.strong_text_color = ui.visuals().hyperlink_color;
            for child in node.children() {
                render_inline_with_job(&mut job, state, child);
            }

            // We can't render a link as part of a larger job, so we do it here
            // while we have `ui` instead of `job`.
            ui.hyperlink_to(job, &link_node.url)
                .on_hover_text(&link_node.url);
        }
        _ => {
            let mut job = egui::text::LayoutJob::default();
            render_inline_no_recurse(&mut job, &mut state, node);
            ui.label(job);

            for child in node.children() {
                render_inline_with_ui(ui, state, child);
            }
        }
    }
}

fn render_inline_with_job<'a>(
    job: &mut egui::text::LayoutJob,
    mut state: InlineFormatState<'_>,
    node: &'a comrak::nodes::AstNode<'a>,
) {
    render_inline_no_recurse(job, &mut state, node);
    render_children_inline_with_job(job, state, node);
}

fn render_children_inline_with_job<'a>(
    job: &mut egui::text::LayoutJob,
    state: InlineFormatState<'_>,
    node: &'a comrak::nodes::AstNode<'a>,
) {
    for child in node.children() {
        render_inline_with_job(job, state, child);
    }
}

fn render_inline_no_recurse(
    job: &mut egui::text::LayoutJob,
    state: &mut InlineFormatState<'_>,
    node: &comrak::nodes::AstNode<'_>,
) {
    match &node.data.borrow().value {
        comrak::nodes::NodeValue::Text(s) => job.append(s, 0.0, state.text_format()),
        comrak::nodes::NodeValue::SoftBreak => job.append(" ", 0.0, state.text_format()),
        comrak::nodes::NodeValue::LineBreak => job.append("\n", 0.0, state.text_format()),
        comrak::nodes::NodeValue::Code(code_node) => {
            state.code = true;
            job.append(&code_node.literal, 0.0, state.text_format());
        }
        comrak::nodes::NodeValue::HtmlInline(_) => append_not_implemented(job, state, "HtmlInline"),
        comrak::nodes::NodeValue::Raw(_) => append_not_implemented(job, state, "Raw"),
        comrak::nodes::NodeValue::Emph => state.italics = true,
        comrak::nodes::NodeValue::Strong => state.bold = true,
        comrak::nodes::NodeValue::Strikethrough => state.strikethrough = true,
        comrak::nodes::NodeValue::Superscript => state.superscript = true,
        comrak::nodes::NodeValue::Link(_) => append_not_implemented(job, state, "Link"),
        comrak::nodes::NodeValue::Image(_) => append_not_implemented(job, state, "Image"),
        comrak::nodes::NodeValue::FootnoteReference(_) => {
            append_not_implemented(job, state, "FootnoteReference");
        }
        comrak::nodes::NodeValue::Math(_) => append_not_implemented(job, state, "Math"),
        comrak::nodes::NodeValue::MultilineBlockQuote(_) => todo!(),
        comrak::nodes::NodeValue::Escaped => (),
        comrak::nodes::NodeValue::WikiLink(_) => append_not_implemented(job, state, "WikiLink"),
        comrak::nodes::NodeValue::Underline => state.underline = true,
        comrak::nodes::NodeValue::Subscript => state.subscript = true,
        comrak::nodes::NodeValue::SpoileredText => {
            append_not_implemented(job, state, "SpoileredText");
        }
        comrak::nodes::NodeValue::EscapedTag(_) => append_not_implemented(job, state, "EscapedTag"),

        _ => (), // ignore block nodes
    }
}

fn not_implemented_label(ui: &mut egui::Ui, feature: &str) {
    ui.colored_label(
        ui.visuals().error_fg_color,
        format!("{feature} is not implemented"),
    );
}
fn append_not_implemented(
    job: &mut egui::text::LayoutJob,
    state: &InlineFormatState<'_>,
    feature: &str,
) {
    let mut text_format = state.text_format();
    text_format.color = egui::Color32::RED;
    job.append(&format!("{feature} is not implemented"), 0.0, text_format);
}
