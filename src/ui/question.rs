use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};
use ratatui::Frame;

use crate::model::QuestionKind;
use crate::state::AppState;
use crate::ui::markdown::body_elements_to_lines;

pub fn draw_question(f: &mut Frame, area: Rect, state: &AppState) {
    let Some(question) = state.current_question() else {
        let p = Paragraph::new("No questions").block(
            Block::default().borders(Borders::ALL),
        );
        f.render_widget(p, area);
        return;
    };

    let mut lines: Vec<Line> = Vec::new();

    // Question header
    lines.push(Line::from(Span::styled(
        format!("  ## {}. {}", question.number, question.title),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    // Question body
    let body_lines = body_elements_to_lines(&question.body_lines);
    for line in body_lines {
        let indented = Line::from(
            std::iter::once(Span::raw("  "))
                .chain(line.spans.into_iter())
                .collect::<Vec<_>>(),
        );
        lines.push(indented);
    }

    // Answer widget
    let qnum = question.number;
    match &question.kind {
        QuestionKind::SingleChoice(choices) => {
            lines.push(Line::from(""));
            for (i, choice) in choices.iter().enumerate() {
                let is_selected = state.is_choice_selected(qnum, choice.label);
                let is_cursor = i == state.choice_cursor;

                let radio = if is_selected { "(●)" } else { "( )" };
                let cursor = if is_cursor { "▸ " } else { "  " };

                let style = if is_cursor {
                    Style::default().fg(Color::Yellow)
                } else if is_selected {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                };

                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(cursor.to_string(), style),
                    Span::styled(format!("{} ", radio), style),
                    Span::styled(
                        format!("{}) {}", choice.label, choice.text),
                        style,
                    ),
                ]));
            }
        }
        QuestionKind::MultiChoice(choices) => {
            lines.push(Line::from(""));
            for (i, choice) in choices.iter().enumerate() {
                let is_selected = state.is_choice_selected(qnum, choice.label);
                let is_cursor = i == state.choice_cursor;

                let checkbox = if is_selected { "[x]" } else { "[ ]" };
                let cursor = if is_cursor { "▸ " } else { "  " };

                let style = if is_cursor {
                    Style::default().fg(Color::Yellow)
                } else if is_selected {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                };

                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(cursor.to_string(), style),
                    Span::styled(format!("{} ", checkbox), style),
                    Span::styled(
                        format!("{}) {}", choice.label, choice.text),
                        style,
                    ),
                ]));
            }
        }
        QuestionKind::Short => {
            lines.push(Line::from(""));
            let answer_text = state
                .answers
                .get(&qnum)
                .and_then(|a| a.text.as_ref())
                .cloned()
                .unwrap_or_default();

            let display_text = if state.input_mode == crate::state::InputMode::TextInput {
                &state.text_input
            } else {
                &answer_text
            };

            // Input box
            lines.push(Line::from(vec![
                Span::raw("  ┌"),
                Span::raw("─".repeat(area.width.saturating_sub(6) as usize)),
                Span::raw("┐"),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  │ "),
                Span::styled(
                    if display_text.is_empty() {
                        "Type your answer...".to_string()
                    } else {
                        display_text.to_string()
                    },
                    if display_text.is_empty() {
                        Style::default().fg(Color::DarkGray)
                    } else {
                        Style::default().fg(Color::White)
                    },
                ),
                Span::raw(" ".repeat(
                    area.width
                        .saturating_sub(6 + display_text.len().min(area.width as usize - 6) as u16)
                        as usize,
                )),
                Span::raw("│"),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  └"),
                Span::raw("─".repeat(area.width.saturating_sub(6) as usize)),
                Span::raw("┘"),
            ]));
        }
        QuestionKind::Long => {
            lines.push(Line::from(""));
            let answer_text = state
                .answers
                .get(&qnum)
                .and_then(|a| a.text.as_ref())
                .cloned()
                .unwrap_or_default();

            if answer_text.is_empty() {
                lines.push(Line::from(Span::styled(
                    "  No content yet",
                    Style::default().fg(Color::DarkGray),
                )));
            } else {
                let preview_lines: Vec<&str> = answer_text.lines().take(5).collect();
                for pl in &preview_lines {
                    lines.push(Line::from(Span::styled(
                        format!("  │ {}", pl),
                        Style::default().fg(Color::White),
                    )));
                }
                let total_lines = answer_text.lines().count();
                if total_lines > 5 {
                    lines.push(Line::from(Span::styled(
                        format!("  │ ... ({} more lines)", total_lines - 5),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  [Ctrl+E] Open editor",
                Style::default().fg(Color::DarkGray),
            )));
        }
        QuestionKind::File(constraints) => {
            lines.push(Line::from(""));
            let files = state.get_file_list(qnum);

            if files.is_empty() {
                lines.push(Line::from(Span::styled(
                    "  No files attached",
                    Style::default().fg(Color::DarkGray),
                )));
            } else {
                for (i, file) in files.iter().enumerate() {
                    let is_cursor = i == state.file_cursor;
                    let filename = std::path::Path::new(file)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                    let style = if is_cursor {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    };
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(
                            if is_cursor { "▸ " } else { "  " }.to_string(),
                            style,
                        ),
                        Span::styled(format!("📎 {}", filename), style),
                    ]));
                }
            }

            lines.push(Line::from(""));

            // Show constraints
            let mut constraint_parts: Vec<String> = Vec::new();
            if let Some(max) = constraints.max_files {
                constraint_parts.push(format!("max {} files", max));
            }
            if let Some(max) = constraints.max_size {
                let mb = max / (1024 * 1024);
                constraint_parts.push(format!("max {}MB", mb));
            }
            if !constraints.accept.is_empty() {
                constraint_parts.push(format!("accept: {}", constraints.accept.join(", ")));
            }
            if !constraint_parts.is_empty() {
                lines.push(Line::from(Span::styled(
                    format!("  ({})", constraint_parts.join(", ")),
                    Style::default().fg(Color::DarkGray),
                )));
            }

            lines.push(Line::from(Span::styled(
                "  [Ctrl+A] Attach file   [Ctrl+D] Delete selected",
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    // Hints
    let revealed = state.hints_revealed.get(&qnum).copied().unwrap_or(0);
    let total_hints = question.hints.len();
    if total_hints > 0 {
        lines.push(Line::from(""));

        // Show revealed hints
        for i in 0..revealed.min(total_hints) {
            lines.push(Line::from(Span::styled(
                format!("  💡 Hint {}: {}", i + 1, question.hints[i]),
                Style::default().fg(Color::Yellow),
            )));
        }

        let remaining = total_hints.saturating_sub(revealed);
        if remaining > 0 {
            lines.push(Line::from(Span::styled(
                format!("  [Ctrl+H] Show hint ({} available)", remaining),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    // Apply scroll with clamping
    let total_content_lines = lines.len();
    let visible_height = area.height as usize;
    let scroll = state.question_scroll.min(total_content_lines.saturating_sub(visible_height));
    let display_lines: Vec<Line> = lines.into_iter().skip(scroll).collect();

    let widget = Paragraph::new(display_lines)
        .wrap(Wrap { trim: false });
    f.render_widget(widget, area);

    // Scrollbar
    if total_content_lines > visible_height {
        let mut scrollbar_state = ScrollbarState::new(total_content_lines)
            .position(scroll)
            .viewport_content_length(visible_height);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}
