use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct AppLayout {
    pub titlebar: Rect,
    pub sidebar: Rect,
    pub main: Rect,
    pub statusbar: Rect,
    pub keybar: Rect,
}

pub fn compute_layout(area: Rect) -> AppLayout {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // titlebar
            Constraint::Min(5),    // middle (sidebar + main)
            Constraint::Length(1), // statusbar
            Constraint::Length(1), // keybar
        ])
        .split(area);

    let middle = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(30), // sidebar (icon + number + title)
            Constraint::Min(20),    // main content
        ])
        .split(vertical[1]);

    AppLayout {
        titlebar: vertical[0],
        sidebar: middle[0],
        main: middle[1],
        statusbar: vertical[2],
        keybar: vertical[3],
    }
}
