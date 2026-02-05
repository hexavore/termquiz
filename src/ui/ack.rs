use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::state::{AckFocus, AppState};

pub fn draw_preamble(f: &mut Frame, area: Rect, state: &AppState) {
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            &state.quiz.title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for text in &state.quiz.preamble {
        lines.push(Line::from(text.as_str()));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press Enter to continue",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));

    let block = Block::default().borders(Borders::ALL);
    let widget = Paragraph::new(lines)
        .block(block)
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(Wrap { trim: false });
    f.render_widget(widget, area);
}

pub fn draw_acknowledgment(f: &mut Frame, area: Rect, state: &AppState) {
    let ack_text = state
        .quiz
        .frontmatter
        .acknowledgment
        .as_ref()
        .and_then(|a| a.text.as_ref())
        .cloned()
        .unwrap_or_else(|| "No acknowledgment text.".to_string());

    let name_style = if state.ack_focus == AckFocus::Name {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };

    let checkbox_style = if state.ack_focus == AckFocus::Checkbox {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };

    let ok_enabled = state.ack_name.len() >= 2 && state.ack_checkbox;
    let ok_style = if state.ack_focus == AckFocus::Ok {
        if ok_enabled {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        }
    } else if ok_enabled {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let cancel_style = if state.ack_focus == AckFocus::Cancel {
        Style::default()
            .fg(Color::Red)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let checkbox_icon = if state.ack_checkbox { "[x]" } else { "[ ]" };

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            &state.quiz.title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "─".repeat(area.width.saturating_sub(4) as usize),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
    ];

    // Acknowledgment text
    for line in ack_text.lines() {
        lines.push(Line::from(format!("  {}", line.trim())));
    }

    lines.extend_from_slice(&[
        Line::from(""),
        Line::from(Span::styled(
            "─".repeat(area.width.saturating_sub(4) as usize),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from("  Type your full name to acknowledge:"),
        Line::from(""),
    ]);

    // Name input box
    let name_display = if state.ack_name.is_empty() && state.ack_focus != AckFocus::Name {
        String::new()
    } else {
        state.ack_name.clone()
    };

    let box_width = area.width.saturating_sub(8) as usize;
    let name_padded = format!(
        "{:<width$}",
        name_display,
        width = box_width
    );

    lines.push(Line::from(vec![
        Span::raw("  ┌"),
        Span::raw("─".repeat(box_width)),
        Span::raw("┐"),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  │"),
        Span::styled(name_padded, name_style),
        Span::raw("│"),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  └"),
        Span::raw("─".repeat(box_width)),
        Span::raw("┘"),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!(
            "  {} I have read and agree to the above statement",
            checkbox_icon
        ),
        checkbox_style,
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("                "),
        Span::styled("[ OK ]", ok_style),
        Span::raw("              "),
        Span::styled("[ Cancel ]", cancel_style),
    ]));
    lines.push(Line::from(""));

    let block = Block::default().borders(Borders::ALL);
    let widget = Paragraph::new(lines).block(block);
    f.render_widget(widget, area);
}
