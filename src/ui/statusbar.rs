use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::state::AppState;

pub fn draw_statusbar(f: &mut Frame, area: Rect, state: &AppState) {
    let counts = state.status_counts();

    let line = Line::from(vec![
        Span::raw(" "),
        Span::styled(
            format!("✓ {} done", counts.done),
            Style::default().fg(Color::Green),
        ),
        Span::raw("   "),
        Span::styled(
            format!("◐ {} partial", counts.partial),
            Style::default().fg(Color::Blue),
        ),
        Span::raw("   "),
        Span::styled(
            format!("⚑ {} flagged", counts.flagged),
            Style::default().fg(Color::Red),
        ),
        Span::raw("   "),
        Span::styled(
            format!("○ {} empty", counts.empty),
            Style::default().fg(Color::White),
        ),
        Span::raw("   "),
        Span::styled(
            format!("· {} unread", counts.unread),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw("   "),
        Span::styled(
            "[?] help",
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let widget = Paragraph::new(line).style(Style::default().bg(Color::Rgb(30, 30, 30)));
    f.render_widget(widget, area);
}
