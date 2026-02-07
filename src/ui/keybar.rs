use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::model::QuestionKind;
use crate::state::{AppState, InputMode, MainFocus};

pub fn draw_keybar(f: &mut Frame, area: Rect, state: &AppState) {
    let is_long = state
        .current_question()
        .map_or(false, |q| matches!(q.kind, QuestionKind::Long));

    let bindings: Vec<(&str, &str)> = if state.main_focus != MainFocus::Answer
        && state.input_mode != InputMode::AckNameInput
    {
        vec![
            ("Tab", "next"),
            ("Space", "press"),
            ("Ctrl+S", "submit"),
            ("Ctrl+Q", "quit"),
        ]
    } else {
        match state.input_mode {
            InputMode::TextInput if is_long => vec![
                ("↑/↓", "move line"),
                ("Ctrl+←/→", "prev/next Q"),
                ("Esc", "done editing"),
                ("Ctrl+E", "ext. editor"),
                ("Tab", "next"),
                ("Ctrl+S", "submit"),
                ("Ctrl+Q", "quit"),
            ],
            InputMode::TextInput => vec![
                ("←/→", "cursor"),
                ("Ctrl+←/→", "prev/next Q"),
                ("Esc", "done editing"),
                ("Tab", "next"),
                ("Ctrl+S", "submit"),
                ("Ctrl+Q", "quit"),
            ],
            InputMode::ChoiceSelect => vec![
                ("a-z", "answer"),
                ("arrows", "prev/next"),
                ("PgUp/PgDn", "jump 5"),
                ("Tab", "next"),
                ("Ctrl+N", "done"),
                ("Ctrl+F", "flag"),
                ("Ctrl+S", "submit"),
                ("Ctrl+Q", "quit"),
            ],
            InputMode::Navigation => vec![
                ("arrows", "prev/next"),
                ("PgUp/PgDn", "jump 5"),
                ("Tab", "next"),
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
        }
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
