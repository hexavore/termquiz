use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::state::AppState;
use crate::timer::format_duration;

pub fn draw_titlebar(f: &mut Frame, area: Rect, state: &AppState) {
    let title = &state.quiz.title;

    let timer_text = if let Some(secs) = state.remaining_seconds {
        let formatted = format!(" {} remaining ", format_duration(secs));
        if secs <= 120 && secs > 0 {
            Span::styled(
                formatted,
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                formatted,
                Style::default()
                    .fg(Color::Rgb(200, 200, 120)),
            )
        }
    } else {
        Span::raw("")
    };

    let title_text = format!("[ {} ]", title);
    let title_span = Span::styled(
        title_text.clone(),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );

    // Center the title: pad left so title sits in the middle of the full width
    let available = area.width as usize;
    let timer_len = if state.remaining_seconds.is_some() {
        format_duration(state.remaining_seconds.unwrap_or(0)).len() + 13
    } else {
        0
    };
    let title_len = title_text.len();
    let center_pad = if available > title_len {
        (available - title_len) / 2
    } else {
        0
    };
    // Right padding fills the gap between centered title and right-aligned timer
    let right_pad = available.saturating_sub(center_pad + title_len + timer_len);

    let line = Line::from(vec![
        Span::raw(" ".repeat(center_pad)),
        title_span,
        Span::raw(" ".repeat(right_pad)),
        timer_text,
    ]);

    let widget = Paragraph::new(line)
        .style(Style::default().bg(Color::DarkGray))
        .alignment(Alignment::Left);
    f.render_widget(widget, area);
}
