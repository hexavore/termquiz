use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

pub fn markdown_to_lines(text: &str) -> Vec<Line<'static>> {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(text, opts);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut style_stack: Vec<Style> = vec![Style::default()];

    for event in parser {
        match event {
            Event::Start(Tag::Paragraph) => {
                current_spans.clear();
            }
            Event::End(TagEnd::Paragraph) => {
                if !current_spans.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut current_spans)));
                }
                lines.push(Line::from(""));
            }
            Event::Start(Tag::Strong) => {
                let current = *style_stack.last().unwrap_or(&Style::default());
                style_stack.push(current.add_modifier(Modifier::BOLD));
            }
            Event::End(TagEnd::Strong) => {
                style_stack.pop();
            }
            Event::Start(Tag::Emphasis) => {
                let current = *style_stack.last().unwrap_or(&Style::default());
                style_stack.push(current.add_modifier(Modifier::ITALIC));
            }
            Event::End(TagEnd::Emphasis) => {
                style_stack.pop();
            }
            Event::Start(Tag::List(_)) => {}
            Event::End(TagEnd::List(_)) => {}
            Event::Start(Tag::Item) => {
                current_spans.clear();
                current_spans.push(Span::raw("  • "));
            }
            Event::End(TagEnd::Item) => {
                if !current_spans.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut current_spans)));
                }
            }
            Event::Start(Tag::CodeBlock(_)) => {
                current_spans.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                // Code block lines already added
            }
            Event::Start(Tag::Heading { level, .. }) => {
                current_spans.clear();
                let prefix = match level {
                    pulldown_cmark::HeadingLevel::H1 => "# ",
                    pulldown_cmark::HeadingLevel::H2 => "## ",
                    pulldown_cmark::HeadingLevel::H3 => "### ",
                    _ => "",
                };
                current_spans.push(Span::styled(
                    prefix.to_string(),
                    Style::default().add_modifier(Modifier::BOLD),
                ));
            }
            Event::End(TagEnd::Heading(_)) => {
                if !current_spans.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut current_spans)));
                }
                lines.push(Line::from(""));
            }
            Event::Text(text) => {
                let style = *style_stack.last().unwrap_or(&Style::default());
                // For code blocks, split by newlines
                let t = text.to_string();
                if style_stack.len() == 1 {
                    current_spans.push(Span::styled(t, style));
                } else {
                    current_spans.push(Span::styled(t, style));
                }
            }
            Event::Code(code) => {
                current_spans.push(Span::styled(
                    format!("`{}`", code),
                    Style::default().fg(Color::Yellow),
                ));
            }
            Event::SoftBreak => {
                if !current_spans.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut current_spans)));
                }
            }
            Event::HardBreak => {
                if !current_spans.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut current_spans)));
                }
            }
            Event::Rule => {
                lines.push(Line::from(Span::styled(
                    "─".repeat(40),
                    Style::default().fg(Color::DarkGray),
                )));
            }
            _ => {}
        }
    }

    if !current_spans.is_empty() {
        lines.push(Line::from(current_spans));
    }

    lines
}

pub fn body_elements_to_lines(elements: &[crate::model::BodyElement]) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for elem in elements {
        match elem {
            crate::model::BodyElement::Text(text) => {
                // Parse inline markdown
                let parsed = markdown_to_lines(text);
                lines.extend(parsed);
            }
            crate::model::BodyElement::Code(code) => {
                for code_line in code.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", code_line),
                        Style::default().fg(Color::Green),
                    )));
                }
                lines.push(Line::from(""));
            }
            crate::model::BodyElement::Bold(text) => {
                lines.push(Line::from(Span::styled(
                    text.clone(),
                    Style::default().add_modifier(Modifier::BOLD),
                )));
            }
            crate::model::BodyElement::Italic(text) => {
                lines.push(Line::from(Span::styled(
                    text.clone(),
                    Style::default().add_modifier(Modifier::ITALIC),
                )));
            }
            crate::model::BodyElement::InlineCode(text) => {
                lines.push(Line::from(Span::styled(
                    format!("`{}`", text),
                    Style::default().fg(Color::Yellow),
                )));
            }
            crate::model::BodyElement::ListItem(text) => {
                lines.push(Line::from(vec![
                    Span::raw("  • "),
                    Span::raw(text.clone()),
                ]));
            }
        }
    }
    lines
}
