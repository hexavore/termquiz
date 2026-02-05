use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::Frame;

use crate::state::{ActivePanel, AppState, QuestionStatus};

pub fn draw_sidebar(f: &mut Frame, area: Rect, state: &AppState) {
    let mut lines: Vec<Line> = Vec::new();

    let visible_height = area.height.saturating_sub(2) as usize; // account for borders
    let current = state.current_question;
    let total_questions = state.quiz.questions.len();

    // Auto-scroll sidebar
    let scroll_offset = if current >= state.sidebar_scroll + visible_height {
        current.saturating_sub(visible_height - 1)
    } else if current < state.sidebar_scroll {
        current
    } else {
        state.sidebar_scroll
    };

    let title_max_len = area.width.saturating_sub(10) as usize; // leave room for cursor+icon+number

    for (i, q) in state.quiz.questions.iter().enumerate().skip(scroll_offset) {
        if lines.len() >= visible_height {
            break;
        }

        let status = state.question_status(q.number);
        let (icon, color) = match status {
            QuestionStatus::Unread => ("·", Color::DarkGray),
            QuestionStatus::Empty => ("○", Color::White),
            QuestionStatus::Partial => ("◐", Color::Blue),
            QuestionStatus::Done => ("✓", Color::Green),
            QuestionStatus::Flagged => ("⚑", Color::Red),
        };

        let is_current = i == current;
        let bg = if is_current {
            Color::DarkGray
        } else {
            Color::Reset
        };
        let style = if is_current {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
                .bg(bg)
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
                if is_current { " ▸" } else { "  " }.to_string(),
                style,
            ),
            Span::styled(format!("{} ", icon), Style::default().fg(color).bg(bg)),
            Span::styled(format!("{:>2}. ", q.number), style),
            Span::styled(title_display, style),
        ]);
        lines.push(line);
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
    if total_questions > visible_height {
        let mut scrollbar_state = ScrollbarState::new(total_questions.saturating_sub(1))
            .position(current)
            .viewport_content_length(3);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        let scrollbar_area = area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        });
        f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }
}
