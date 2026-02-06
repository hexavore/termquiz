use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::Frame;

use crate::model::QuestionKind;
use crate::state::AppState;
use crate::ui::markdown::body_elements_to_lines;

/// Maps content lines to clickable elements for mouse handling.
pub struct QuestionHitMap {
    pub button_line: usize,
    /// (first_content_line, choice_index) for each choice option.
    pub choice_lines: Vec<(usize, usize)>,
}

/// Compute the hit map for the current question, mirroring draw_question's layout.
pub fn compute_hit_map(state: &AppState, area_width: u16) -> Option<QuestionHitMap> {
    let question = state.current_question()?;
    let qnum = question.number;
    let mut line_count: usize = 0;

    // Header: title + blank
    line_count += 2;

    // Body lines
    let body_lines = body_elements_to_lines(&question.body_lines);
    line_count += body_lines.len();

    // Answer widget
    let mut choice_lines: Vec<(usize, usize)> = Vec::new();
    match &question.kind {
        QuestionKind::SingleChoice(choices) | QuestionKind::MultiChoice(choices) => {
            line_count += 1; // blank line before choices
            for (i, choice) in choices.iter().enumerate() {
                choice_lines.push((line_count, i));
                let prefix_len = 10; // "  (●) A. " ≈ 10
                let text_width = (area_width as usize).saturating_sub(prefix_len);
                let wrapped = wrap_text(&choice.text, text_width);
                line_count += wrapped.len();
            }
        }
        QuestionKind::Short => {
            line_count += 1; // blank
            line_count += 3; // input box (top border, content, bottom border)
        }
        QuestionKind::Long => {
            line_count += 1; // blank
            let answer_text = state
                .answers
                .get(&qnum)
                .and_then(|a| a.text.as_ref())
                .cloned()
                .unwrap_or_default();
            if answer_text.is_empty() {
                line_count += 1; // "No content yet"
            } else {
                let preview_count = answer_text.lines().take(5).count();
                line_count += preview_count;
                if answer_text.lines().count() > 5 {
                    line_count += 1; // "... (N more lines)"
                }
            }
            line_count += 1; // blank
            line_count += 1; // "[Ctrl+E] Open editor"
        }
        QuestionKind::File(constraints) => {
            line_count += 1; // blank
            let files = state.get_file_list(qnum);
            if files.is_empty() {
                line_count += 1;
            } else {
                line_count += files.len();
            }
            line_count += 1; // blank
            let mut has_constraints = false;
            if constraints.max_files.is_some()
                || constraints.max_size.is_some()
                || !constraints.accept.is_empty()
            {
                has_constraints = true;
                line_count += 1;
            }
            let _ = has_constraints;
            line_count += 1; // "[Ctrl+A] Attach file"
        }
    }

    // Hints
    let revealed = state.hints_revealed.get(&qnum).copied().unwrap_or(0);
    let total_hints = question.hints.len();
    if total_hints > 0 {
        line_count += 1; // blank
        line_count += revealed.min(total_hints); // revealed hints
        let remaining = total_hints.saturating_sub(revealed);
        if remaining > 0 {
            line_count += 1; // "[Ctrl+H] Show hint"
        }
    }

    // Button row: blank + buttons
    line_count += 1; // blank
    let button_line = line_count;

    Some(QuestionHitMap {
        button_line,
        choice_lines,
    })
}

/// Wrap text to fit within `width` columns, breaking at word boundaries.
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut result = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_string();
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            result.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        result.push(current);
    }
    if result.is_empty() {
        result.push(String::new());
    }
    result
}

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
                let letter = (b'A' + i as u8) as char;

                let radio = if is_selected { "(●)" } else { "( )" };

                let style = if is_selected {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                };

                // Prefix: "  (●) A. " = 9 chars
                let prefix = format!("  {} {}. ", radio, letter);
                let prefix_len = prefix.len();
                let text_width = (area.width as usize).saturating_sub(prefix_len);
                let wrapped = wrap_text(&choice.text, text_width);
                for (li, wline) in wrapped.iter().enumerate() {
                    if li == 0 {
                        lines.push(Line::from(vec![
                            Span::styled(prefix.clone(), style),
                            Span::styled(wline.clone(), style),
                        ]));
                    } else {
                        lines.push(Line::from(vec![
                            Span::raw(" ".repeat(prefix_len)),
                            Span::styled(wline.clone(), style),
                        ]));
                    }
                }
            }
        }
        QuestionKind::MultiChoice(choices) => {
            lines.push(Line::from(""));
            for (i, choice) in choices.iter().enumerate() {
                let is_selected = state.is_choice_selected(qnum, choice.label);
                let letter = (b'A' + i as u8) as char;

                let checkbox = if is_selected { "[x]" } else { "[ ]" };

                let style = if is_selected {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                };

                // Prefix: "  [x] A. " = 9 chars
                let prefix = format!("  {} {}. ", checkbox, letter);
                let prefix_len = prefix.len();
                let text_width = (area.width as usize).saturating_sub(prefix_len);
                let wrapped = wrap_text(&choice.text, text_width);
                for (li, wline) in wrapped.iter().enumerate() {
                    if li == 0 {
                        lines.push(Line::from(vec![
                            Span::styled(prefix.clone(), style),
                            Span::styled(wline.clone(), style),
                        ]));
                    } else {
                        lines.push(Line::from(vec![
                            Span::raw(" ".repeat(prefix_len)),
                            Span::styled(wline.clone(), style),
                        ]));
                    }
                }
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
                for (_i, file) in files.iter().enumerate() {
                    let filename = std::path::Path::new(file)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::raw(format!("📎 {}", filename)),
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
                "  [Ctrl+A] Attach file",
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

    // Done / Flag buttons
    lines.push(Line::from(""));
    let is_done = state.is_done(qnum);
    let is_flagged = state.is_flagged(qnum);

    let done_style = if is_done {
        Style::default().fg(Color::White).bg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray).bg(Color::Rgb(50, 50, 50))
    };
    let done_ul_style = done_style.add_modifier(Modifier::UNDERLINED);
    let flag_style = if is_flagged {
        Style::default().fg(Color::White).bg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray).bg(Color::Rgb(50, 50, 50))
    };
    let flag_ul_style = flag_style.add_modifier(Modifier::UNDERLINED);

    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(" ✓ DO", done_style),
        Span::styled("N", done_ul_style),
        Span::styled("E ", done_style),
        Span::raw("  "),
        Span::styled(" ⚑ ", flag_style),
        Span::styled("F", flag_ul_style),
        Span::styled("LAG ", flag_style),
    ]));

    // Apply scroll with clamping
    let total_content_lines = lines.len();
    let visible_height = area.height as usize;
    let scroll = state.question_scroll.min(total_content_lines.saturating_sub(visible_height));
    let display_lines: Vec<Line> = lines.into_iter().skip(scroll).collect();

    let widget = Paragraph::new(display_lines);
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
