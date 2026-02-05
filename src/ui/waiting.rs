use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::state::AppState;
use crate::timer::{format_wait_duration, time_until_start};

pub fn draw_waiting(f: &mut Frame, area: Rect, state: &AppState) {
    let secs = time_until_start(&state.quiz.frontmatter.start);
    let duration_str = format_wait_duration(secs);

    let lines = vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            &state.quiz.title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("Quiz opens in {}", duration_str),
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "[Ctrl+Q] Exit",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
    ];

    let block = Block::default().borders(Borders::ALL);
    let widget = Paragraph::new(lines)
        .block(block)
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(widget, area);
}

pub fn draw_closed(f: &mut Frame, area: Rect, _state: &AppState) {
    let lines = vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "✗  Quiz Closed",
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("The submission deadline has passed."),
        Line::from(""),
        Line::from(Span::styled(
            "[Enter] Exit",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
    ];

    let block = Block::default().borders(Borders::ALL);
    let widget = Paragraph::new(lines)
        .block(block)
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(widget, area);
}
