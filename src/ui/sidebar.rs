use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::Frame;

use crate::state::{ActivePanel, AppState, QuestionStatus};

const STATUS_ROWS: usize = 6; // 1 separator + 5 status lines

pub fn draw_sidebar(f: &mut Frame, area: Rect, state: &AppState) {
    let mut lines: Vec<Line> = Vec::new();

    let inner_height = area.height.saturating_sub(2) as usize; // account for top/bottom border
    let question_height = inner_height.saturating_sub(STATUS_ROWS);
    let current = state.current_question;
    let total_questions = state.quiz.questions.len();

    // Auto-scroll sidebar
    let scroll_offset = if current >= state.sidebar_scroll + question_height {
        current.saturating_sub(question_height - 1)
    } else if current < state.sidebar_scroll {
        current
    } else {
        state.sidebar_scroll
    };

    let title_max_len = area.width.saturating_sub(11) as usize; // cursor+space+icon+space+number+dot+space

    for (i, q) in state.quiz.questions.iter().enumerate().skip(scroll_offset) {
        if lines.len() >= question_height {
            break;
        }

        let status = state.question_status(q.number);
        let (icon, color) = match status {
            QuestionStatus::Unread => ("·", Color::DarkGray),
            QuestionStatus::NotAnswered => ("○", Color::White),
            QuestionStatus::Answered => ("◐", Color::LightBlue),
            QuestionStatus::Done => ("✓", Color::Green),
            QuestionStatus::Flagged => ("⚑", Color::Red),
        };

        let is_current = i == current;
        let bg = if is_current {
            Color::DarkGray
        } else {
            Color::Reset
        };
        let row_fg = match status {
            QuestionStatus::Done => Some(Color::Green),
            QuestionStatus::Flagged => Some(Color::Red),
            _ => None,
        };
        let style = if is_current {
            let s = Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(bg);
            if let Some(fg) = row_fg { s.fg(fg) } else { s.fg(Color::White) }
        } else if let Some(fg) = row_fg {
            Style::default().fg(fg).bg(bg)
        } else {
            Style::default().bg(bg)
        };

        // Truncate title to fit
        let title: String = q.title.chars().take(title_max_len).collect();
        let title_display = if q.title.len() > title_max_len {
            format!("{}…", &title[..title.len().saturating_sub(1)])
        } else {
            title
        };

        // Format: cursor + icon + number + title
        let line = Line::from(vec![
            Span::styled(
                if is_current { " ▸ " } else { "   " }.to_string(),
                style,
            ),
            Span::styled(format!("{} ", icon), if matches!(status, QuestionStatus::Done) {
                Style::default().fg(color).bg(bg).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(color).bg(bg)
            }),
            Span::styled(format!("{:>2}. ", q.number), style),
            Span::styled(title_display, style),
        ]);
        lines.push(line);
    }

    // Pad remaining question area with blank lines
    while lines.len() < question_height {
        lines.push(Line::from(""));
    }

    // Separator line
    let sep_width = area.width.saturating_sub(1) as usize; // -1 for right border
    lines.push(Line::from(Span::styled(
        "─".repeat(sep_width),
        Style::default().fg(Color::DarkGray),
    )));

    // Status counts
    let counts = state.status_counts();
    let max_n = *[counts.done, counts.answered, counts.flagged, counts.not_answered, counts.unread]
        .iter()
        .max()
        .unwrap_or(&0);
    let width = if max_n >= 100 { 3 } else if max_n >= 10 { 2 } else { 1 };

    let status_items: Vec<(&str, usize, Color)> = vec![
        ("✓", counts.done, Color::Green),
        ("◐", counts.answered, Color::LightBlue),
        ("⚑", counts.flagged, Color::Red),
        ("○", counts.not_answered, Color::White),
        ("·", counts.unread, Color::DarkGray),
    ];
    let labels = ["done", "answered", "flagged", "not answered", "unread"];

    for (idx, ((icon, count, color), label)) in status_items.iter().zip(labels.iter()).enumerate() {
        let _ = idx;
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(icon.to_string(), Style::default().fg(*color)),
            Span::styled(format!(" {:>w$}", count, w = width), Style::default().fg(*color)),
            Span::styled(format!(" {}", label), Style::default().fg(*color)),
        ]));
    }

    let border_style = if state.active_panel == ActivePanel::Sidebar {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let block = Block::default()
        .borders(Borders::RIGHT)
        .title(format!(" {} Questions ", total_questions))
        .title_style(Style::default().add_modifier(Modifier::BOLD))
        .border_style(border_style);

    let widget = Paragraph::new(lines).block(block);
    f.render_widget(widget, area);

    // Scrollbar — tracks current_question so it stays in sync with drag
    if total_questions > question_height {
        let scrollbar_area = Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: question_height as u16,
        };
        let mut scrollbar_state = ScrollbarState::new(total_questions.saturating_sub(1))
            .position(current)
            .viewport_content_length(3);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }
}
