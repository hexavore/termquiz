use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::state::AppState;

pub fn draw_already_submitted(f: &mut Frame, area: Rect, state: &AppState) {
    let submitted_at = state
        .submitted_at
        .as_deref()
        .unwrap_or("unknown");

    let lines = vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "✓ Quiz Already Submitted",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!("Submitted: {}", submitted_at)),
        Line::from(""),
        Line::from("You cannot modify your submission."),
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

pub fn draw_pushing(f: &mut Frame, area: Rect, _state: &AppState) {
    let lines = vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "Submitting...",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Pushing to git remote..."),
        Line::from(""),
    ];

    let block = Block::default().borders(Borders::ALL);
    let widget = Paragraph::new(lines)
        .block(block)
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(widget, area);
}

pub fn draw_push_retrying(f: &mut Frame, area: Rect, state: &AppState) {
    let timeout_remaining = 600u32.saturating_sub(state.push_elapsed_secs);
    let timeout_min = timeout_remaining / 60;
    let timeout_sec = timeout_remaining % 60;

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "⚠  Submission Failed — Retrying",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Could not reach git server."),
        Line::from(""),
        Line::from(format!(
            "Attempt {}    Retrying in {}s...    [{:02}:{:02} until timeout]",
            state.push_attempt, state.push_retry_secs, timeout_min, timeout_sec
        )),
        Line::from(""),
        Line::from(Span::styled(
            "[Esc] Cancel and keep working",
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

pub fn draw_save_local(f: &mut Frame, area: Rect, state: &AppState) {
    let repo_display = state.repo_dir.display().to_string();

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "✗  Submission Failed — Saved Locally",
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Your answers have been saved to:"),
        Line::from(format!("{}/response/", repo_display)),
        Line::from(""),
        Line::from("To submit manually, run:"),
        Line::from(""),
        Line::from(Span::styled(
            format!("  cd {}", repo_display),
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            "  git add response/",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            "  git commit -m \"termquiz: manual submit\"",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            "  git push",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(""),
        Line::from("Contact your instructor if you need assistance."),
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
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(Wrap { trim: false });
    f.render_widget(widget, area);
}

pub fn draw_done(f: &mut Frame, area: Rect, state: &AppState) {
    let lines = vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "✓  Quiz Submitted Successfully",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!(
            "Submitted: {}",
            state.submitted_at.as_deref().unwrap_or("just now")
        )),
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
