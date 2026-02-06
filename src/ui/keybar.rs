use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::state::{AppState, InputMode};

pub fn draw_keybar(f: &mut Frame, area: Rect, state: &AppState) {
    let bindings: Vec<(&str, &str)> = match state.input_mode {
        InputMode::TextInput => vec![
            ("←/→", "cursor"),
            ("Esc", "done editing"),
            ("Tab", "panel"),
            ("Ctrl+S", "submit"),
            ("Ctrl+Q", "quit"),
        ],
        InputMode::ChoiceSelect => vec![
            ("a-z", "answer"),
            ("arrows", "prev/next"),
            ("PgUp/PgDn", "jump 5"),
            ("Tab", "panel"),
            ("Ctrl+N", "done"),
            ("Ctrl+F", "flag"),
            ("Ctrl+S", "submit"),
            ("Ctrl+Q", "quit"),
        ],
        InputMode::Navigation => vec![
            ("arrows", "prev/next"),
            ("PgUp/PgDn", "jump 5"),
            ("Tab", "panel"),
            ("Ctrl+N", "done"),
            ("Ctrl+F", "flag"),
            ("Ctrl+S", "submit"),
            ("Ctrl+Q", "quit"),
        ],
        InputMode::AckNameInput => vec![
            ("Tab", "next field"),
            ("Enter", "confirm"),
            ("Esc", "cancel"),
        ],
    };

    let mut spans: Vec<Span> = vec![Span::raw(" ")];
    for (i, (key, action)) in bindings.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("   "));
        }
        spans.push(Span::styled(
            key.to_string(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(format!(" {}", action)));
    }

    let line = Line::from(spans);
    let widget = Paragraph::new(line).style(Style::default().bg(Color::Rgb(20, 20, 20)));
    f.render_widget(widget, area);
}
