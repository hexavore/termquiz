use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::state::{AppState, Dialog};

pub fn draw_dialog(f: &mut Frame, area: Rect, state: &AppState) {
    let Some(dialog) = state.top_dialog() else {
        return;
    };

    match dialog {
        Dialog::ConfirmSubmit => draw_confirm_submit(f, area, state),
        Dialog::ConfirmQuit => draw_confirm_quit(f, area, state),
        Dialog::ConfirmHint => draw_confirm_hint(f, area, state),
        Dialog::DoneRequiresAnswer => draw_done_requires_answer(f, area),
        Dialog::TwoMinuteWarning => draw_two_minute_warning(f, area),
        Dialog::Help => draw_help(f, area),
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

fn draw_confirm_submit(f: &mut Frame, area: Rect, state: &AppState) {
    let counts = state.status_counts();
    let mut msg_lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled(
            "   Submit your quiz?",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if counts.not_answered + counts.unread > 0 {
        msg_lines.push(Line::from(Span::styled(
            format!(
                "   {} questions are not answered.",
                counts.not_answered + counts.unread
            ),
            Style::default().fg(Color::White),
        )));
    }
    if counts.flagged > 0 {
        msg_lines.push(Line::from(Span::styled(
            format!("   {} questions are flagged.", counts.flagged),
            Style::default().fg(Color::White),
        )));
    }

    msg_lines.push(Line::from(""));
    msg_lines.push(Line::from(vec![
        Span::styled(
            "   [Enter] Confirm",
            Style::default().fg(Color::Green),
        ),
        Span::raw("    "),
        Span::styled("[Esc] Cancel", Style::default().fg(Color::DarkGray)),
    ]));
    msg_lines.push(Line::from(""));

    let rect = centered_rect(42, msg_lines.len() as u16, area);
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let widget = Paragraph::new(msg_lines).block(block);
    f.render_widget(widget, rect);
}

fn draw_confirm_quit(f: &mut Frame, area: Rect, _state: &AppState) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "   Quit?",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("   Progress is saved locally."),
        Line::from(""),
        Line::from(vec![
            Span::styled("   [Enter] Confirm", Style::default().fg(Color::Green)),
            Span::raw("    "),
            Span::styled("[Esc] Cancel", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
    ];

    let rect = centered_rect(38, lines.len() as u16, area);
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let widget = Paragraph::new(lines).block(block);
    f.render_widget(widget, rect);
}

fn draw_confirm_hint(f: &mut Frame, area: Rect, _state: &AppState) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "   Reveal hint?",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("   This will be recorded."),
        Line::from(""),
        Line::from(vec![
            Span::styled("   [Enter] Confirm", Style::default().fg(Color::Green)),
            Span::raw("    "),
            Span::styled("[Esc] Cancel", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
    ];

    let rect = centered_rect(38, lines.len() as u16, area);
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let widget = Paragraph::new(lines).block(block);
    f.render_widget(widget, rect);
}

fn draw_done_requires_answer(f: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "   Cannot mark as done",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("   Answer the question first."),
        Line::from(""),
        Line::from(Span::styled(
            "           [OK]",
            Style::default().fg(Color::Green),
        )),
        Line::from(""),
    ];

    let rect = centered_rect(38, lines.len() as u16, area);
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let widget = Paragraph::new(lines).block(block);
    f.render_widget(widget, rect);
}

fn draw_two_minute_warning(f: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "   ⚠  2 MINUTES REMAINING",
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("   Your quiz will auto-submit when"),
        Line::from("   time expires. Save your work."),
        Line::from(""),
        Line::from(Span::styled(
            "          [Enter] Continue",
            Style::default().fg(Color::Green),
        )),
        Line::from(""),
    ];

    let rect = centered_rect(42, lines.len() as u16, area);
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));
    let widget = Paragraph::new(lines).block(block);
    f.render_widget(widget, rect);
}

fn draw_help(f: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "   Key Bindings",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("   arrows     Previous/Next question"),
        Line::from("   PgUp/PgDn  Jump 5 questions"),
        Line::from("   Home/End   First/Last question"),
        Line::from("   a-z        Select/toggle choice"),
        Line::from("   Tab        Switch panel"),
        Line::from("   Ctrl+N     Toggle done mark"),
        Line::from("   Ctrl+H     Reveal next hint"),
        Line::from("   Ctrl+F     Toggle flag"),
        Line::from("   Ctrl+E     Open editor (long)"),
        Line::from("   Ctrl+A     Attach file"),
        Line::from("   Ctrl+S     Submit quiz"),
        Line::from("   Ctrl+Q     Quit (saves state)"),
        Line::from("   ?          This help"),
        Line::from("   Esc        Close dialog"),
        Line::from(""),
        Line::from(Span::styled(
            "        [Esc] Close",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
    ];

    let rect = centered_rect(44, lines.len() as u16, area);
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Help ")
        .border_style(Style::default().fg(Color::Cyan));
    let widget = Paragraph::new(lines).block(block);
    f.render_widget(widget, rect);
}
