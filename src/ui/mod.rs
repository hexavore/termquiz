pub mod ack;
pub mod dialog;
pub mod keybar;
pub mod layout;
pub mod markdown;
pub mod question;
pub mod result;
pub mod sidebar;
pub mod titlebar;
pub mod waiting;

use ratatui::Frame;

use crate::state::{AppState, Screen};

pub fn draw(f: &mut Frame, state: &AppState) {
    let area = f.area();

    match state.screen {
        Screen::Waiting => {
            waiting::draw_waiting(f, area, state);
        }
        Screen::Preamble => {
            ack::draw_preamble(f, area, state);
        }
        Screen::Acknowledgment => {
            ack::draw_acknowledgment(f, area, state);
        }
        Screen::Working => {
            draw_working(f, area, state);
        }
        Screen::Closed => {
            waiting::draw_closed(f, area, state);
        }
        Screen::AlreadySubmitted => {
            result::draw_already_submitted(f, area, state);
        }
        Screen::Pushing => {
            result::draw_pushing(f, area, state);
        }
        Screen::PushRetrying => {
            result::draw_push_retrying(f, area, state);
        }
        Screen::SaveLocal => {
            result::draw_save_local(f, area, state);
        }
        Screen::Done => {
            result::draw_done(f, area, state);
        }
    }
}

fn draw_working(f: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let layout = layout::compute_layout(area);

    titlebar::draw_titlebar(f, layout.titlebar, state);
    sidebar::draw_sidebar(f, layout.sidebar, state);
    question::draw_question(f, layout.main, state);
    keybar::draw_keybar(f, layout.keybar, state);

    // Draw dialog overlay if any
    if state.has_dialog() {
        dialog::draw_dialog(f, area, state);
    }
}
