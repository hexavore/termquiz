use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::Frame;

use crate::model::QuestionKind;
use crate::state::{AppState, MainFocus};
use crate::ui::markdown::body_elements_to_lines;

/// Maps content lines to clickable elements for mouse handling.
pub struct QuestionHitMap {
    pub button_line: usize,
    /// (first_content_line, choice_index) for each choice option.
    pub choice_lines: Vec<(usize, usize)>,
}

/// Compute the hit map for the current question, mirroring draw_question's layout.
pub fn compute_hit_map(state: &AppState, area: Rect) -> Option<QuestionHitMap> {
    let area_width = area.width;
    let question = state.current_question()?;
    let qnum = question.number;
    let mut line_count: usize = 0;

    // Header: title + blank
    line_count += 2;

    // Body lines (wrapped)
    let body_lines = body_elements_to_lines(&question.body_lines);
    let body_wrap_width = (area_width as usize).saturating_sub(4);
    for bl in body_lines {
        line_count += wrap_styled_line(bl, body_wrap_width).len();
    }

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
            line_count += 1; // blank before editor
            let before_count = line_count;
            let mut after_count = 0;
            let total_hints = question.hints.len();
            if total_hints > 0 {
                after_count += 1;
                let rev = state.hints_revealed.get(&qnum).copied().unwrap_or(0);
                after_count += rev.min(total_hints);
                if total_hints.saturating_sub(rev) > 0 {
                    after_count += 1;
                }
            }
            after_count += 2; // blank + buttons
            let editor_inner = (area.height as usize)
                .saturating_sub(before_count)
                .saturating_sub(2) // top + bottom border
                .saturating_sub(after_count)
                .max(1);
            line_count += 2 + editor_inner; // borders + visible editor rows
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

/// Wrap a styled Line at `width`, preserving span styles across breaks.
fn wrap_styled_line(line: Line<'static>, width: usize) -> Vec<Line<'static>> {
    if width == 0 {
        return vec![line];
    }

    // Compute total display width
    let total_width: usize = line.spans.iter().map(|s| s.content.len()).sum();
    if total_width <= width {
        return vec![line];
    }

    // Flatten into (char, style) pairs
    let mut chars: Vec<(char, Style)> = Vec::new();
    for span in &line.spans {
        for c in span.content.chars() {
            chars.push((c, span.style));
        }
    }

    let mut result: Vec<Line<'static>> = Vec::new();
    let mut pos = 0;

    while pos < chars.len() {
        if chars.len() - pos <= width {
            result.push(styled_chars_to_line(&chars[pos..]));
            break;
        }

        let chunk_end = pos + width;
        let break_at = if chunk_end < chars.len() && chars[chunk_end].0 == ' ' {
            chunk_end
        } else if let Some(sp) = chars[pos..chunk_end].iter().rposition(|(c, _)| *c == ' ') {
            if sp > 0 { pos + sp } else { chunk_end }
        } else {
            chunk_end
        };

        result.push(styled_chars_to_line(&chars[pos..break_at]));
        pos = break_at;
        if pos < chars.len() && chars[pos].0 == ' ' {
            pos += 1;
        }
    }

    if result.is_empty() {
        result.push(Line::from(""));
    }

    result
}

/// Rebuild a Line from (char, style) pairs, grouping consecutive same-style chars into spans.
fn styled_chars_to_line(chars: &[(char, Style)]) -> Line<'static> {
    if chars.is_empty() {
        return Line::from("");
    }

    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut current_text = String::new();
    let mut current_style = chars[0].1;

    for &(c, style) in chars {
        if style == current_style {
            current_text.push(c);
        } else {
            if !current_text.is_empty() {
                spans.push(Span::styled(current_text, current_style));
                current_text = String::new();
            }
            current_style = style;
            current_text.push(c);
        }
    }
    if !current_text.is_empty() {
        spans.push(Span::styled(current_text, current_style));
    }

    Line::from(spans)
}

/// Word-wrap a line, returning (start_char_offset, display_text) for each visual row.
fn wrap_with_offsets(text: &str, width: usize) -> Vec<(usize, String)> {
    if text.is_empty() {
        return vec![(0, String::new())];
    }
    if width == 0 {
        return vec![(0, text.to_string())];
    }

    let mut result: Vec<(usize, String)> = Vec::new();
    let mut pos = 0;
    let bytes = text.as_bytes();

    while pos < text.len() {
        let remaining_len = text.len() - pos;
        if remaining_len <= width {
            result.push((pos, text[pos..].to_string()));
            break;
        }

        // Check if char right after the chunk is a space (natural break)
        if bytes[pos + width] == b' ' {
            result.push((pos, text[pos..pos + width].to_string()));
            pos += width + 1; // skip the space
        } else if let Some(sp) = text[pos..pos + width].rfind(' ') {
            if sp > 0 {
                result.push((pos, text[pos..pos + sp].to_string()));
                pos += sp + 1; // skip the space
            } else {
                // Only a leading space — hard break
                result.push((pos, text[pos..pos + width].to_string()));
                pos += width;
            }
        } else {
            // No space found, hard break
            result.push((pos, text[pos..pos + width].to_string()));
            pos += width;
        }
    }

    if result.is_empty() {
        result.push((0, String::new()));
    }

    result
}

/// Find the visual (row_within_line, col) for a cursor at `cursor_col` in a wrapped line.
fn find_visual_cursor(wraps: &[(usize, String)], cursor_col: usize) -> (usize, usize) {
    for (i, (start, text)) in wraps.iter().enumerate() {
        let next_start = if i + 1 < wraps.len() {
            wraps[i + 1].0
        } else {
            usize::MAX
        };
        if cursor_col < next_start || i == wraps.len() - 1 {
            return (i, cursor_col.saturating_sub(*start).min(text.len()));
        }
    }
    (0, 0)
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

    // Question body (with wrapping)
    let body_lines = body_elements_to_lines(&question.body_lines);
    let body_wrap_width = (area.width as usize).saturating_sub(4); // 2 indent left + 2 margin right
    for line in body_lines {
        let wrapped = wrap_styled_line(line, body_wrap_width);
        for wline in wrapped {
            let indented = Line::from(
                std::iter::once(Span::raw("  "))
                    .chain(wline.spans.into_iter())
                    .collect::<Vec<_>>(),
            );
            lines.push(indented);
        }
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

            // Input box: "  ┌───┐" / "  │ text │" / "  └───┘"
            // 2 margin left + 2 margin right: frame is W-4 wide
            // dashes = frame - 2 (corners) = W - 6
            // inner = frame - 4 ("│ " + " │") = W - 8
            let dashes = area.width.saturating_sub(6) as usize;
            let inner = area.width.saturating_sub(8) as usize;
            let is_editing = state.input_mode == crate::state::InputMode::TextInput;

            lines.push(Line::from(vec![
                Span::raw("  ┌"),
                Span::raw("─".repeat(dashes)),
                Span::raw("┐"),
            ]));

            if display_text.is_empty() && !is_editing {
                // Placeholder
                let placeholder = "Type your answer...";
                let ph_len = placeholder.len().min(inner);
                let padding = inner.saturating_sub(ph_len);
                lines.push(Line::from(vec![
                    Span::raw("  │ "),
                    Span::styled(placeholder, Style::default().fg(Color::DarkGray)),
                    Span::raw(" ".repeat(padding)),
                    Span::raw(" │"),
                ]));
            } else {
                // Text with cursor
                let display_len = display_text.len().min(inner);
                let cursor_pos = if is_editing {
                    state.text_cursor.min(display_len)
                } else {
                    display_len // no cursor shown
                };

                let mut spans = vec![Span::raw("  │ ")];
                if is_editing {
                    let before = &display_text[..cursor_pos];
                    if cursor_pos < display_len {
                        let at_cursor = &display_text[cursor_pos..cursor_pos + 1];
                        let after = &display_text[cursor_pos + 1..display_len];
                        spans.push(Span::styled(before.to_string(), Style::default().fg(Color::White)));
                        spans.push(Span::styled(
                            at_cursor.to_string(),
                            Style::default().fg(Color::Black).bg(Color::White),
                        ));
                        spans.push(Span::styled(after.to_string(), Style::default().fg(Color::White)));
                    } else {
                        spans.push(Span::styled(before.to_string(), Style::default().fg(Color::White)));
                        // Cursor at end — show block cursor on a space
                        spans.push(Span::styled(
                            " ".to_string(),
                            Style::default().fg(Color::Black).bg(Color::White),
                        ));
                    }
                    let visible_len = if cursor_pos < display_len { display_len } else { display_len + 1 };
                    let padding = inner.saturating_sub(visible_len);
                    spans.push(Span::raw(" ".repeat(padding)));
                } else {
                    spans.push(Span::styled(
                        display_text[..display_len].to_string(),
                        Style::default().fg(Color::White),
                    ));
                    let padding = inner.saturating_sub(display_len);
                    spans.push(Span::raw(" ".repeat(padding)));
                }
                spans.push(Span::raw(" │"));
                lines.push(Line::from(spans));
            }

            lines.push(Line::from(vec![
                Span::raw("  └"),
                Span::raw("─".repeat(dashes)),
                Span::raw("┘"),
            ]));
        }
        QuestionKind::Long => {
            lines.push(Line::from(""));

            let is_editing = state.input_mode == crate::state::InputMode::TextInput;
            let display_text = if is_editing {
                state.text_input.clone()
            } else {
                state
                    .answers
                    .get(&qnum)
                    .and_then(|a| a.text.as_ref())
                    .cloned()
                    .unwrap_or_default()
            };

            // Pre-compute lines after editor (hints + buttons)
            let mut after_count: usize = 0;
            if question.hints.len() > 0 {
                after_count += 1; // blank
                let rev = state.hints_revealed.get(&qnum).copied().unwrap_or(0);
                after_count += rev.min(question.hints.len());
                if question.hints.len().saturating_sub(rev) > 0 {
                    after_count += 1;
                }
            }
            after_count += 2; // blank + buttons

            let before_count = lines.len();
            let editor_inner = (area.height as usize)
                .saturating_sub(before_count)
                .saturating_sub(2) // top + bottom border
                .saturating_sub(after_count)
                .max(1);

            let dashes = area.width.saturating_sub(6) as usize;
            let inner_w = area.width.saturating_sub(8) as usize;

            // Split text into logical lines
            let text_lines: Vec<&str> = if display_text.is_empty() {
                vec![""]
            } else {
                display_text.split('\n').collect()
            };

            // Compute cursor logical position
            let (cursor_row, cursor_col) = if is_editing {
                let pos = state.text_cursor.min(state.text_input.len());
                let before = &state.text_input[..pos];
                let row = before.matches('\n').count();
                let col = before.rfind('\n').map_or(pos, |p| pos - p - 1);
                (row, col)
            } else {
                (0, 0)
            };

            // Build visual rows with word wrapping
            let mut visual_rows: Vec<String> = Vec::new();
            let mut cursor_vrow: usize = 0;
            let mut cursor_vcol: usize = 0;

            for (li, line_text) in text_lines.iter().enumerate() {
                let wraps = wrap_with_offsets(line_text, inner_w);
                if is_editing && li == cursor_row {
                    let (vr, vc) = find_visual_cursor(&wraps, cursor_col);
                    cursor_vrow = visual_rows.len() + vr;
                    cursor_vcol = vc;
                }
                for (_offset, display) in wraps {
                    visual_rows.push(display);
                }
            }

            // Location indicator
            let current_line = if is_editing {
                cursor_row + 1
            } else if !display_text.is_empty() {
                1
            } else {
                0
            };
            let total_logical = text_lines.len();
            let indicator = if current_line > 0 {
                format!("[line {} of {}]", current_line, total_logical)
            } else {
                String::new()
            };

            // Top border with indicator
            let left_dashes = dashes.saturating_sub(indicator.len());
            lines.push(Line::from(vec![
                Span::raw("  ┌"),
                Span::raw("─".repeat(left_dashes)),
                Span::styled(indicator.clone(), Style::default().fg(Color::DarkGray)),
                Span::raw("┐"),
            ]));

            // Compute scroll based on cursor visual row
            let scroll = if cursor_vrow >= editor_inner {
                cursor_vrow - editor_inner + 1
            } else {
                0
            };

            // Render visible rows
            for vi in 0..editor_inner {
                let row_idx = scroll + vi;
                if row_idx < visual_rows.len() {
                    let row_text = &visual_rows[row_idx];
                    let display_len = row_text.len().min(inner_w);

                    if is_editing && row_idx == cursor_vrow {
                        let col = cursor_vcol.min(display_len);
                        let mut spans = vec![Span::raw("  │ ")];
                        let before_cursor = &row_text[..col];
                        if col < display_len {
                            let at_cursor = &row_text[col..col + 1];
                            let after_cursor = &row_text[col + 1..display_len];
                            spans.push(Span::styled(
                                before_cursor.to_string(),
                                Style::default().fg(Color::White),
                            ));
                            spans.push(Span::styled(
                                at_cursor.to_string(),
                                Style::default().fg(Color::Black).bg(Color::White),
                            ));
                            spans.push(Span::styled(
                                after_cursor.to_string(),
                                Style::default().fg(Color::White),
                            ));
                            let padding = inner_w.saturating_sub(display_len);
                            spans.push(Span::raw(" ".repeat(padding)));
                        } else {
                            spans.push(Span::styled(
                                before_cursor.to_string(),
                                Style::default().fg(Color::White),
                            ));
                            spans.push(Span::styled(
                                " ".to_string(),
                                Style::default().fg(Color::Black).bg(Color::White),
                            ));
                            let padding = inner_w.saturating_sub(display_len + 1);
                            spans.push(Span::raw(" ".repeat(padding)));
                        }
                        spans.push(Span::raw(" │"));
                        lines.push(Line::from(spans));
                    } else if row_idx == 0 && !is_editing && display_text.is_empty() {
                        let placeholder = "Type your answer...";
                        let ph_len = placeholder.len().min(inner_w);
                        let padding = inner_w.saturating_sub(ph_len);
                        lines.push(Line::from(vec![
                            Span::raw("  │ "),
                            Span::styled(placeholder, Style::default().fg(Color::DarkGray)),
                            Span::raw(" ".repeat(padding)),
                            Span::raw(" │"),
                        ]));
                    } else {
                        let padding = inner_w.saturating_sub(display_len);
                        lines.push(Line::from(vec![
                            Span::raw("  │ "),
                            Span::styled(
                                row_text[..display_len].to_string(),
                                Style::default().fg(Color::White),
                            ),
                            Span::raw(" ".repeat(padding)),
                            Span::raw(" │"),
                        ]));
                    }
                } else {
                    lines.push(Line::from(vec![
                        Span::raw("  │ "),
                        Span::raw(" ".repeat(inner_w)),
                        Span::raw(" │"),
                    ]));
                }
            }

            // Bottom border
            lines.push(Line::from(vec![
                Span::raw("  └"),
                Span::raw("─".repeat(dashes)),
                Span::raw("┘"),
            ]));
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
            let hint_focused = state.main_focus == MainFocus::Hint;
            let marker = if hint_focused { " ▸" } else { "  " };
            let hint_style = if hint_focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            lines.push(Line::from(Span::styled(
                format!("{} [Ctrl+H] Show hint ({} available)", marker, remaining),
                hint_style,
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

    let done_marker = if state.main_focus == MainFocus::DoneButton { " ▸" } else { "  " };
    let flag_marker = if state.main_focus == MainFocus::FlagButton { " ▸" } else { "  " };

    lines.push(Line::from(vec![
        Span::raw(done_marker),
        Span::styled(" ✓ DO", done_style),
        Span::styled("N", done_ul_style),
        Span::styled("E ", done_style),
        Span::raw(flag_marker),
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
