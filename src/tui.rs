use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::layout::Rect;
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;

use crate::editor;
use crate::model::QuestionKind;
use crate::persist;
use crate::state::*;
use crate::submit;
use crate::timer::TimerEvent;
use crate::git;

#[derive(Debug)]
pub enum PushEvent {
    Success,
    Retrying {
        attempt: u32,
        wait_secs: u32,
        elapsed: u32,
        error: String,
    },
    Timeout,
    Cancelled,
    Conflict(String),
}

pub fn run_tui(
    mut state: AppState,
    timer_rx: mpsc::Receiver<TimerEvent>,
    state_dir: std::path::PathBuf,
) -> Result<(), String> {
    enable_raw_mode().map_err(|e| format!("Cannot enable raw mode: {}", e))?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .map_err(|e| format!("Cannot enter alternate screen: {}", e))?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        Terminal::new(backend).map_err(|e| format!("Cannot create terminal: {}", e))?;

    // Mark first question as visited
    if !state.quiz.questions.is_empty() {
        let qnum = state.quiz.questions[0].number;
        state.visited.insert(qnum, true);
        state.load_text_input_for_current();
        // Set initial input mode
        if let Some(q) = state.current_question() {
            match &q.kind {
                QuestionKind::SingleChoice(_) | QuestionKind::MultiChoice(_) => {
                    state.input_mode = InputMode::ChoiceSelect;
                }
                QuestionKind::Short | QuestionKind::Long => {
                    state.input_mode = InputMode::TextInput;
                }
                _ => {
                    state.input_mode = InputMode::Navigation;
                }
            }
        }
    }

    let push_cancel = Arc::new(AtomicBool::new(false));
    let (push_tx, push_rx) = mpsc::channel::<PushEvent>();

    let result = main_loop(
        &mut terminal,
        &mut state,
        &timer_rx,
        &push_rx,
        &push_tx,
        &push_cancel,
        &state_dir,
    );

    // Restore terminal
    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture).ok();

    result
}

fn main_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut AppState,
    timer_rx: &mpsc::Receiver<TimerEvent>,
    push_rx: &mpsc::Receiver<PushEvent>,
    push_tx: &mpsc::Sender<PushEvent>,
    push_cancel: &Arc<AtomicBool>,
    state_dir: &std::path::Path,
) -> Result<(), String> {
    loop {
        terminal
            .draw(|f| crate::ui::draw(f, state))
            .map_err(|e| format!("Draw error: {}", e))?;

        if state.should_quit {
            break;
        }

        // Poll for input events
        if event::poll(Duration::from_millis(100))
            .map_err(|e| format!("Poll error: {}", e))?
        {
            match event::read().map_err(|e| format!("Read error: {}", e))? {
                Event::Key(key) => {
                    handle_key(key, state, terminal, push_tx, push_cancel, state_dir)?;
                    // Auto-save after key handling
                    if state.screen == Screen::Working {
                        let _ = persist::save_state(state, state_dir);
                    }
                }
                Event::Mouse(mouse) => {
                    let size = terminal.size().unwrap_or_default();
                    let area = Rect::new(0, 0, size.width, size.height);
                    handle_mouse(mouse, state, area)?;
                    // Auto-save after mouse handling
                    if state.screen == Screen::Working {
                        let _ = persist::save_state(state, state_dir);
                    }
                }
                _ => {}
            }
        }

        // Handle timer events
        while let Ok(ev) = timer_rx.try_recv() {
            handle_timer(ev, state, push_tx, push_cancel, state_dir)?;
        }

        // Handle push events
        while let Ok(ev) = push_rx.try_recv() {
            handle_push(ev, state, state_dir)?;
        }
    }

    Ok(())
}

fn handle_key(
    key: KeyEvent,
    state: &mut AppState,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    push_tx: &mpsc::Sender<PushEvent>,
    push_cancel: &Arc<AtomicBool>,
    state_dir: &std::path::Path,
) -> Result<(), String> {
    // Handle dialog keys first
    if state.has_dialog() {
        return handle_dialog_key(key, state, terminal, push_tx, push_cancel, state_dir);
    }

    match state.screen {
        Screen::Waiting => handle_waiting_key(key, state),
        Screen::Preamble => handle_preamble_key(key, state),
        Screen::Acknowledgment => handle_ack_key(key, state),
        Screen::Working => {
            handle_working_key(key, state, terminal, push_tx, push_cancel, state_dir)
        }
        Screen::Closed | Screen::AlreadySubmitted | Screen::Done | Screen::SaveLocal => {
            if key.code == KeyCode::Enter {
                state.should_quit = true;
            }
            Ok(())
        }
        Screen::PushRetrying => {
            if key.code == KeyCode::Esc {
                push_cancel.store(true, Ordering::SeqCst);
                state.screen = Screen::Working;
            }
            Ok(())
        }
        Screen::Pushing => Ok(()),
    }
}

fn handle_waiting_key(key: KeyEvent, state: &mut AppState) -> Result<(), String> {
    if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
        state.should_quit = true;
    }
    Ok(())
}

fn handle_preamble_key(key: KeyEvent, state: &mut AppState) -> Result<(), String> {
    match key.code {
        KeyCode::Enter => {
            let needs_ack = state
                .quiz
                .frontmatter
                .acknowledgment
                .as_ref()
                .map(|a| a.required)
                .unwrap_or(false);

            if needs_ack && state.ack_data.is_none() {
                state.screen = Screen::Acknowledgment;
                state.input_mode = InputMode::AckNameInput;
                state.ack_focus = AckFocus::Name;
            } else {
                state.screen = Screen::Working;
                if state.started_at.is_none() {
                    state.started_at = Some(chrono::Utc::now().to_rfc3339());
                }
            }
        }
        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.should_quit = true;
        }
        _ => {}
    }
    Ok(())
}

fn handle_ack_key(key: KeyEvent, state: &mut AppState) -> Result<(), String> {
    match state.ack_focus {
        AckFocus::Name => match key.code {
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                state.ack_name.push(c);
            }
            KeyCode::Backspace => {
                state.ack_name.pop();
            }
            KeyCode::Tab => {
                state.ack_focus = AckFocus::Checkbox;
            }
            KeyCode::Enter => {
                state.ack_focus = AckFocus::Checkbox;
            }
            KeyCode::Esc => {
                state.screen = Screen::Preamble;
                state.input_mode = InputMode::Navigation;
            }
            _ => {}
        },
        AckFocus::Checkbox => match key.code {
            KeyCode::Char(' ') | KeyCode::Enter => {
                state.ack_checkbox = !state.ack_checkbox;
            }
            KeyCode::Tab => {
                state.ack_focus = AckFocus::Ok;
            }
            KeyCode::Esc => {
                state.screen = Screen::Preamble;
                state.input_mode = InputMode::Navigation;
            }
            _ => {}
        },
        AckFocus::Ok => match key.code {
            KeyCode::Enter => {
                if state.ack_name.len() >= 2 && state.ack_checkbox {
                    let ack_text = state
                        .quiz
                        .frontmatter
                        .acknowledgment
                        .as_ref()
                        .and_then(|a| a.text.as_ref())
                        .cloned()
                        .unwrap_or_default();

                    state.ack_data = Some(crate::model::AckData {
                        name: state.ack_name.clone(),
                        agreed_at: chrono::Utc::now().to_rfc3339(),
                        text_hash: persist::compute_str_hash(&ack_text),
                    });
                    state.screen = Screen::Working;
                    state.input_mode = InputMode::Navigation;
                    if state.started_at.is_none() {
                        state.started_at = Some(chrono::Utc::now().to_rfc3339());
                    }
                }
            }
            KeyCode::Tab => {
                state.ack_focus = AckFocus::Cancel;
            }
            KeyCode::Esc => {
                state.screen = Screen::Preamble;
                state.input_mode = InputMode::Navigation;
            }
            _ => {}
        },
        AckFocus::Cancel => match key.code {
            KeyCode::Enter => {
                state.screen = Screen::Preamble;
                state.input_mode = InputMode::Navigation;
            }
            KeyCode::Tab => {
                state.ack_focus = AckFocus::Name;
                state.input_mode = InputMode::AckNameInput;
            }
            KeyCode::Esc => {
                state.screen = Screen::Preamble;
                state.input_mode = InputMode::Navigation;
            }
            _ => {}
        },
    }
    Ok(())
}

fn handle_working_key(
    key: KeyEvent,
    state: &mut AppState,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    _push_tx: &mpsc::Sender<PushEvent>,
    _push_cancel: &Arc<AtomicBool>,
    state_dir: &std::path::Path,
) -> Result<(), String> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    // Global bindings
    if ctrl {
        match key.code {
            KeyCode::Char('q') => {
                state.push_dialog(Dialog::ConfirmQuit);
                return Ok(());
            }
            KeyCode::Char('s') => {
                state.save_current_text_input();
                state.push_dialog(Dialog::ConfirmSubmit);
                return Ok(());
            }
            KeyCode::Char('n') => {
                if !state.toggle_done() {
                    state.push_dialog(Dialog::DoneRequiresAnswer);
                }
                return Ok(());
            }
            KeyCode::Char('f') => {
                state.toggle_flag();
                return Ok(());
            }
            KeyCode::Up | KeyCode::Left => {
                state.save_current_text_input();
                navigate_prev(state);
                return Ok(());
            }
            KeyCode::Down | KeyCode::Right => {
                state.save_current_text_input();
                navigate_next(state);
                return Ok(());
            }
            KeyCode::Char('h') => {
                let qnum = state.current_question_number();
                if let Some(q) = state.current_question() {
                    let revealed = state.hints_revealed.get(&qnum).copied().unwrap_or(0);
                    if revealed < q.hints.len() {
                        state.push_dialog(Dialog::ConfirmHint);
                    }
                }
                return Ok(());
            }
            KeyCode::Char('e') => {
                if let Some(q) = state.current_question() {
                    if matches!(q.kind, QuestionKind::Long) {
                        let qnum = q.number;
                        let current_text = state
                            .answers
                            .get(&qnum)
                            .and_then(|a| a.text.as_ref())
                            .cloned()
                            .unwrap_or_default();

                        // Suspend terminal
                        disable_raw_mode_safe();
                        execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture).ok();

                        match editor::open_editor(&current_text) {
                            Ok(new_text) => {
                                state.answers.insert(
                                    qnum,
                                    crate::model::Answer {
                                        answer_type: "long".to_string(),
                                        selected: None,
                                        text: Some(new_text),
                                        files: None,
                                    },
                                );
                                state.load_text_input_for_current();
                            }
                            Err(_e) => {
                                // Editor failed, keep old content
                            }
                        }

                        // Restore terminal
                        execute!(terminal.backend_mut(), EnterAlternateScreen, EnableMouseCapture).ok();
                        enable_raw_mode().ok();
                        terminal.clear().ok();
                    }
                }
                return Ok(());
            }
            KeyCode::Char('a') => {
                if let Some(q) = state.current_question().cloned() {
                    if let QuestionKind::File(ref constraints) = q.kind {
                        // Check max files
                        let current_files = state.get_file_list(q.number);
                        if let Some(max) = constraints.max_files {
                            if current_files.len() >= max as usize {
                                return Ok(());
                            }
                        }

                        // Suspend terminal for zenity
                        disable_raw_mode_safe();
                        execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture).ok();

                        let file_result = editor::pick_file();

                        // Restore terminal
                        execute!(terminal.backend_mut(), EnterAlternateScreen, EnableMouseCapture).ok();
                        enable_raw_mode().ok();
                        terminal.clear().ok();

                        match file_result {
                            Ok(Some(path)) => {
                                // Validate
                                if let Err(_e) = editor::validate_file(
                                    &path,
                                    constraints.max_size,
                                    &constraints.accept,
                                ) {
                                    // Validation error - could show in status
                                    return Ok(());
                                }
                                // Copy to state dir
                                match editor::copy_file_to_state(&path, state_dir, q.number) {
                                    Ok(dest) => {
                                        state.add_file(q.number, dest);
                                    }
                                    Err(_) => {}
                                }
                            }
                            Ok(None) => {}
                            Err(ref e) if e == "zenity_unavailable" => {
                                // Fall back to TUI text input for file path
                                // For now, skip
                            }
                            Err(_) => {}
                        }
                    }
                }
                return Ok(());
            }
            _ => {}
        }
    }

    // Tab cycles focus within the main panel
    if key.code == KeyCode::Tab {
        state.cycle_main_focus();
        return Ok(());
    }

    // Space activates focused hint/button (when not on Answer)
    if key.code == KeyCode::Char(' ') && !ctrl && state.main_focus != MainFocus::Answer {
        match state.main_focus {
            MainFocus::Hint => {
                let qnum = state.current_question_number();
                if let Some(q) = state.current_question() {
                    let revealed = state.hints_revealed.get(&qnum).copied().unwrap_or(0);
                    if revealed < q.hints.len() {
                        state.push_dialog(Dialog::ConfirmHint);
                    }
                }
            }
            MainFocus::DoneButton => {
                if !state.toggle_done() {
                    state.push_dialog(Dialog::DoneRequiresAnswer);
                }
            }
            MainFocus::FlagButton => {
                state.toggle_flag();
            }
            MainFocus::Answer => {}
        }
        return Ok(());
    }

    // Input-mode-specific bindings
    match state.input_mode {
        InputMode::TextInput => handle_text_input_key(key, state),
        InputMode::ChoiceSelect => handle_choice_key(key, state),
        InputMode::Navigation => handle_nav_key(key, state),
        _ => Ok(()),
    }
}

fn handle_text_input_key(key: KeyEvent, state: &mut AppState) -> Result<(), String> {
    let is_long = state
        .current_question()
        .map_or(false, |q| matches!(q.kind, QuestionKind::Long));

    match key.code {
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.text_input.insert(state.text_cursor, c);
            state.text_cursor += 1;
        }
        KeyCode::Backspace => {
            if state.text_cursor > 0 {
                state.text_cursor -= 1;
                state.text_input.remove(state.text_cursor);
            }
        }
        KeyCode::Delete => {
            if state.text_cursor < state.text_input.len() {
                state.text_input.remove(state.text_cursor);
            }
        }
        KeyCode::Left => {
            if state.text_cursor > 0 {
                state.text_cursor -= 1;
            }
        }
        KeyCode::Right => {
            if state.text_cursor < state.text_input.len() {
                state.text_cursor += 1;
            }
        }
        KeyCode::Enter => {
            if is_long {
                state.text_input.insert(state.text_cursor, '\n');
                state.text_cursor += 1;
            } else {
                state.save_current_text_input();
                navigate_next(state);
            }
        }
        KeyCode::Up => {
            if is_long {
                move_cursor_up(state);
            } else {
                state.save_current_text_input();
                navigate_prev(state);
            }
        }
        KeyCode::Down => {
            if is_long {
                move_cursor_down(state);
            } else {
                state.save_current_text_input();
                navigate_next(state);
            }
        }
        KeyCode::Home => {
            if is_long {
                let before = &state.text_input[..state.text_cursor];
                let line_start = before.rfind('\n').map_or(0, |p| p + 1);
                state.text_cursor = line_start;
            } else {
                state.text_cursor = 0;
            }
        }
        KeyCode::End => {
            if is_long {
                let after = &state.text_input[state.text_cursor..];
                let line_end = after
                    .find('\n')
                    .map_or(state.text_input.len(), |p| state.text_cursor + p);
                state.text_cursor = line_end;
            } else {
                state.text_cursor = state.text_input.len();
            }
        }
        KeyCode::Esc => {
            state.save_current_text_input();
            state.input_mode = InputMode::Navigation;
        }
        _ => {}
    }
    // If text was emptied, clear done mark immediately
    if state.text_input.is_empty() {
        let qnum = state.current_question_number();
        if state.done_marks.get(&qnum).copied().unwrap_or(false) {
            state.done_marks.insert(qnum, false);
        }
    }
    Ok(())
}

fn cursor_row_col(text: &str, cursor: usize) -> (usize, usize) {
    let pos = cursor.min(text.len());
    let before = &text[..pos];
    let row = before.matches('\n').count();
    let col = before.rfind('\n').map_or(pos, |p| pos - p - 1);
    (row, col)
}

fn move_cursor_up(state: &mut AppState) {
    let (row, col) = cursor_row_col(&state.text_input, state.text_cursor);
    if row == 0 {
        return;
    }
    let lines: Vec<&str> = state.text_input.split('\n').collect();
    let target_row = row - 1;
    let target_col = col.min(lines[target_row].len());
    let mut offset = 0;
    for i in 0..target_row {
        offset += lines[i].len() + 1;
    }
    offset += target_col;
    state.text_cursor = offset;
}

fn move_cursor_down(state: &mut AppState) {
    let (row, col) = cursor_row_col(&state.text_input, state.text_cursor);
    let lines: Vec<&str> = state.text_input.split('\n').collect();
    if row + 1 >= lines.len() {
        return;
    }
    let target_row = row + 1;
    let target_col = col.min(lines[target_row].len());
    let mut offset = 0;
    for i in 0..target_row {
        offset += lines[i].len() + 1;
    }
    offset += target_col;
    state.text_cursor = offset;
}

fn handle_choice_key(key: KeyEvent, state: &mut AppState) -> Result<(), String> {
    if let Some(q) = state.current_question().cloned() {
        match &q.kind {
            QuestionKind::SingleChoice(choices) => match key.code {
                KeyCode::Up | KeyCode::Left => {
                    navigate_prev(state);
                }
                KeyCode::Down | KeyCode::Right => {
                    navigate_next(state);
                }
                KeyCode::Char('?') => {
                    state.push_dialog(Dialog::Help);
                }
                KeyCode::Char(c) if c.is_ascii_lowercase() && !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    let idx = (c as u8 - b'a') as usize;
                    if idx < choices.len() {
                        state.choice_cursor = idx;
                        state.select_single_choice(idx);
                    }
                }
                _ => {
                    handle_page_keys(key, state);
                }
            },
            QuestionKind::MultiChoice(choices) => match key.code {
                KeyCode::Up | KeyCode::Left => {
                    navigate_prev(state);
                }
                KeyCode::Down | KeyCode::Right => {
                    navigate_next(state);
                }
                KeyCode::Char('?') => {
                    state.push_dialog(Dialog::Help);
                }
                KeyCode::Char(c) if c.is_ascii_lowercase() && !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    let idx = (c as u8 - b'a') as usize;
                    if idx < choices.len() {
                        state.choice_cursor = idx;
                        state.toggle_multi_choice(idx);
                    }
                }
                _ => {
                    handle_page_keys(key, state);
                }
            },
            _ => {}
        }
    }
    Ok(())
}

fn handle_nav_key(key: KeyEvent, state: &mut AppState) -> Result<(), String> {
    // Enter or typing a character resumes editing for text questions
    let is_text_question = state.current_question().map_or(false, |q| {
        matches!(q.kind, QuestionKind::Short | QuestionKind::Long)
    });
    if is_text_question {
        match key.code {
            KeyCode::Enter => {
                state.input_mode = InputMode::TextInput;
                return Ok(());
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL) && c != '?' =>
            {
                state.input_mode = InputMode::TextInput;
                state.text_input.insert(state.text_cursor, c);
                state.text_cursor += 1;
                return Ok(());
            }
            _ => {}
        }
    }

    match key.code {
        KeyCode::Up | KeyCode::Left => navigate_prev(state),
        KeyCode::Down | KeyCode::Right => navigate_next(state),
        KeyCode::Char('?') => {
            state.push_dialog(Dialog::Help);
        }
        _ => {
            handle_page_keys(key, state);
        }
    }
    Ok(())
}

fn handle_page_keys(key: KeyEvent, state: &mut AppState) {
    let total = state.quiz.questions.len();
    match key.code {
        KeyCode::PageUp => {
            let new_idx = state.current_question.saturating_sub(5);
            state.navigate_to(new_idx);
        }
        KeyCode::PageDown => {
            let new_idx = (state.current_question + 5).min(total.saturating_sub(1));
            state.navigate_to(new_idx);
        }
        KeyCode::Home => {
            state.navigate_to(0);
        }
        KeyCode::End => {
            if total > 0 {
                state.navigate_to(total - 1);
            }
        }
        _ => {}
    }
}

fn navigate_prev(state: &mut AppState) {
    if state.current_question > 0 {
        state.navigate_to(state.current_question - 1);
    }
}

fn navigate_next(state: &mut AppState) {
    let total = state.quiz.questions.len();
    if state.current_question + 1 < total {
        state.navigate_to(state.current_question + 1);
    }
}

fn handle_dialog_key(
    key: KeyEvent,
    state: &mut AppState,
    _terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    push_tx: &mpsc::Sender<PushEvent>,
    push_cancel: &Arc<AtomicBool>,
    state_dir: &std::path::Path,
) -> Result<(), String> {
    let dialog = state.top_dialog().cloned();
    match dialog {
        Some(Dialog::ConfirmSubmit) => match key.code {
            KeyCode::Enter => {
                state.pop_dialog();
                do_submit(state, push_tx, push_cancel, state_dir)?;
            }
            KeyCode::Esc => {
                state.pop_dialog();
            }
            _ => {}
        },
        Some(Dialog::ConfirmQuit) => match key.code {
            KeyCode::Enter => {
                state.pop_dialog();
                state.save_current_text_input();
                let _ = persist::save_state(state, state_dir);
                state.should_quit = true;
            }
            KeyCode::Esc => {
                state.pop_dialog();
            }
            _ => {}
        },
        Some(Dialog::ConfirmHint) => match key.code {
            KeyCode::Enter => {
                state.pop_dialog();
                let qnum = state.current_question_number();
                let current = state.hints_revealed.get(&qnum).copied().unwrap_or(0);
                state.hints_revealed.insert(qnum, current + 1);
                // If all hints now revealed and focus is on Hint, advance to DoneButton
                if state.main_focus == MainFocus::Hint {
                    let all_revealed = state.current_question().map_or(true, |q| {
                        current + 1 >= q.hints.len()
                    });
                    if all_revealed {
                        state.main_focus = MainFocus::DoneButton;
                    }
                }
            }
            KeyCode::Esc => {
                state.pop_dialog();
            }
            _ => {}
        },
        Some(Dialog::DoneRequiresAnswer) => {
            state.pop_dialog();
        }
        Some(Dialog::TwoMinuteWarning) => match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                state.pop_dialog();
            }
            _ => {}
        },
        Some(Dialog::Help) => match key.code {
            KeyCode::Esc | KeyCode::Char('?') => {
                state.pop_dialog();
            }
            _ => {}
        },
        None => {}
    }
    Ok(())
}

fn handle_timer(
    event: TimerEvent,
    state: &mut AppState,
    push_tx: &mpsc::Sender<PushEvent>,
    push_cancel: &Arc<AtomicBool>,
    state_dir: &std::path::Path,
) -> Result<(), String> {
    match event {
        TimerEvent::Tick(secs) => {
            state.remaining_seconds = Some(secs);

            // Check if we transitioned from waiting
            if state.screen == Screen::Waiting && secs <= 0 {
                // Time to start
                state.screen = Screen::Preamble;
            }
        }
        TimerEvent::TwoMinuteWarning => {
            if state.screen == Screen::Working && !state.has_dialog() {
                state.push_dialog(Dialog::TwoMinuteWarning);
            }
        }
        TimerEvent::TimeExpired => {
            state.remaining_seconds = Some(0);
            if state.screen == Screen::Working {
                state.save_current_text_input();
                do_submit(state, push_tx, push_cancel, state_dir)?;
            }
        }
    }
    Ok(())
}

fn handle_push(
    event: PushEvent,
    state: &mut AppState,
    state_dir: &std::path::Path,
) -> Result<(), String> {
    match event {
        PushEvent::Success => {
            state.screen = Screen::Done;
            // Clear local state
            let _ = persist::clear_state(state_dir);
        }
        PushEvent::Retrying {
            attempt,
            wait_secs,
            elapsed,
            error,
        } => {
            state.screen = Screen::PushRetrying;
            state.push_attempt = attempt;
            state.push_retry_secs = wait_secs;
            state.push_elapsed_secs = elapsed;
            state.push_error = error;
        }
        PushEvent::Timeout => {
            state.screen = Screen::SaveLocal;
        }
        PushEvent::Cancelled => {
            state.screen = Screen::Working;
        }
        PushEvent::Conflict(_msg) => {
            state.screen = Screen::AlreadySubmitted;
        }
    }
    Ok(())
}

fn do_submit(
    state: &mut AppState,
    push_tx: &mpsc::Sender<PushEvent>,
    push_cancel: &Arc<AtomicBool>,
    state_dir: &std::path::Path,
) -> Result<(), String> {
    state.submitted_at = Some(chrono::Utc::now().to_rfc3339());
    state.screen = Screen::Pushing;

    // Build response
    let repo_dir = state.repo_dir.clone();
    submit::build_response(state, &repo_dir)?;

    // Save state
    let _ = persist::save_state(state, state_dir);

    // Git add + commit
    if git::is_git_repo(&repo_dir) {
        let commit_msg = submit::build_commit_message(state);
        git::git_add(&repo_dir, &["response/"])?;
        git::git_commit(&repo_dir, &commit_msg)?;

        // Push in background thread
        let tx = push_tx.clone();
        let cancel = push_cancel.clone();
        let dir = repo_dir.clone();
        cancel.store(false, Ordering::SeqCst);

        thread::spawn(move || {
            push_with_retry(dir, tx, cancel);
        });
    } else {
        // Not a git repo, just save locally
        let _ = push_tx.send(PushEvent::Timeout);
    }

    Ok(())
}

fn push_with_retry(
    repo_dir: std::path::PathBuf,
    tx: mpsc::Sender<PushEvent>,
    cancel: Arc<AtomicBool>,
) {
    let mut attempt = 0u32;
    let mut wait_secs = 2u32;
    let mut elapsed = 0u32;
    let max_elapsed = 600u32; // 10 minutes

    loop {
        if cancel.load(Ordering::SeqCst) {
            let _ = tx.send(PushEvent::Cancelled);
            return;
        }

        attempt += 1;
        match git::git_push(&repo_dir) {
            Ok(()) => {
                let _ = tx.send(PushEvent::Success);
                return;
            }
            Err(e) => {
                if e.starts_with("CONFLICT:") {
                    let _ = tx.send(PushEvent::Conflict(e));
                    return;
                }

                if elapsed >= max_elapsed {
                    let _ = tx.send(PushEvent::Timeout);
                    return;
                }

                let _ = tx.send(PushEvent::Retrying {
                    attempt,
                    wait_secs,
                    elapsed,
                    error: e,
                });

                // Wait with cancellation check
                for _ in 0..wait_secs {
                    if cancel.load(Ordering::SeqCst) {
                        let _ = tx.send(PushEvent::Cancelled);
                        return;
                    }
                    thread::sleep(Duration::from_secs(1));
                    elapsed += 1;
                }

                // Exponential backoff, cap at 30s
                wait_secs = (wait_secs * 2).min(30);
            }
        }
    }
}

fn handle_mouse(mouse: MouseEvent, state: &mut AppState, size: Rect) -> Result<(), String> {
    // Only handle mouse in Working screen
    if state.screen != Screen::Working {
        return Ok(());
    }

    // If a dialog is open, ignore mouse events (or could handle dialog buttons)
    if state.has_dialog() {
        return Ok(());
    }

    let layout = crate::ui::layout::compute_layout(size);

    // Sidebar scrollbar hit zone: last 2 columns (border + 1 col inside)
    let sb_hit_left = layout.sidebar.x + layout.sidebar.width.saturating_sub(2);
    let sb_y_start = layout.sidebar.y + 1;
    let sb_y_end = layout.sidebar.y + layout.sidebar.height.saturating_sub(1);

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let x = mouse.column;
            let y = mouse.row;

            // Click on sidebar scrollbar zone
            if x >= sb_hit_left
                && x < layout.sidebar.x + layout.sidebar.width
                && y >= sb_y_start
                && y < sb_y_end
            {
                state.dragging_scrollbar = true;
                scrollbar_navigate(state, y, sb_y_start, sb_y_end);
            }
            // Click in sidebar content (exclude scrollbar zone)
            else if x >= layout.sidebar.x
                && x < sb_hit_left
                && y >= layout.sidebar.y
                && y < layout.sidebar.y + layout.sidebar.height
            {
                state.dragging_scrollbar = false;
                let relative_y = y.saturating_sub(layout.sidebar.y + 1) as usize;
                let visible_height = layout.sidebar.height.saturating_sub(2) as usize;
                let question_height = visible_height.saturating_sub(6); // 1 separator + 5 status
                let status_start = question_height + 1; // after separator

                if relative_y >= status_start && relative_y < status_start + 5 {
                    // Click in status area — check if on checkbox column
                    let inner_width = layout.sidebar.width.saturating_sub(1) as usize;
                    let rel_x = x.saturating_sub(layout.sidebar.x) as usize;
                    // Checkbox occupies the last 3 chars of inner_width
                    if rel_x >= inner_width.saturating_sub(3) && rel_x < inner_width {
                        let status_idx = relative_y - status_start;
                        state.toggle_status_filter(status_idx);
                    }
                } else if relative_y < question_height {
                    // Click on question list — use filtered list
                    let filtered = state.filtered_questions();
                    let current = state.current_question;
                    let current_filtered_pos = filtered.iter().position(|&i| i == current);

                    let scroll_offset = if let Some(pos) = current_filtered_pos {
                        if pos >= state.sidebar_scroll + question_height {
                            pos.saturating_sub(question_height - 1)
                        } else if pos < state.sidebar_scroll {
                            pos
                        } else {
                            state.sidebar_scroll
                        }
                    } else {
                        state.sidebar_scroll.min(filtered.len().saturating_sub(question_height))
                    };

                    let filtered_click = scroll_offset + relative_y;
                    if filtered_click < filtered.len() {
                        let actual_idx = filtered[filtered_click];
                        state.navigate_to(actual_idx);
                        state.active_panel = ActivePanel::Main;
                    }
                }
            }
            // Click in main area (for choice selection and buttons)
            else if x >= layout.main.x
                && x < layout.main.x + layout.main.width
                && y >= layout.main.y
                && y < layout.main.y + layout.main.height
            {
                state.dragging_scrollbar = false;
                let rel_x = x.saturating_sub(layout.main.x) as usize;
                let visible_y = y.saturating_sub(layout.main.y) as usize;
                let content_line = visible_y + state.question_scroll;

                if let Some(hit_map) = crate::ui::question::compute_hit_map(state, layout.main) {
                    if content_line == hit_map.button_line {
                        // Done button: columns 2..10, Flag button: columns 12..20
                        if (2..10).contains(&rel_x) {
                            if !state.toggle_done() {
                                state.push_dialog(Dialog::DoneRequiresAnswer);
                            }
                        } else if (12..20).contains(&rel_x) {
                            state.toggle_flag();
                        }
                    } else if !hit_map.choice_lines.is_empty() {
                        // Find which choice was clicked (each choice may span multiple wrapped lines)
                        let mut clicked_choice = None;
                        for (ci, &(start, idx)) in hit_map.choice_lines.iter().enumerate() {
                            let end = if ci + 1 < hit_map.choice_lines.len() {
                                hit_map.choice_lines[ci + 1].0
                            } else {
                                // choices end before hints/buttons section
                                hit_map.button_line.saturating_sub(1) // at least the blank before buttons
                            };
                            if content_line >= start && content_line < end {
                                clicked_choice = Some(idx);
                                break;
                            }
                        }
                        if let Some(choice_idx) = clicked_choice {
                            if let Some(q) = state.current_question().cloned() {
                                state.choice_cursor = choice_idx;
                                match &q.kind {
                                    QuestionKind::SingleChoice(_) => {
                                        state.select_single_choice(choice_idx);
                                    }
                                    QuestionKind::MultiChoice(_) => {
                                        state.toggle_multi_choice(choice_idx);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            } else {
                state.dragging_scrollbar = false;
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if state.dragging_scrollbar {
                let y = mouse.row;
                scrollbar_navigate(state, y, sb_y_start, sb_y_end);
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            state.dragging_scrollbar = false;
        }
        MouseEventKind::ScrollUp => {
            let x = mouse.column;
            let y = mouse.row;

            if x >= layout.sidebar.x
                && x < layout.sidebar.x + layout.sidebar.width
                && y >= layout.sidebar.y
                && y < layout.sidebar.y + layout.sidebar.height
            {
                if state.current_question > 0 {
                    state.navigate_to(state.current_question - 1);
                }
            } else if x >= layout.main.x
                && x < layout.main.x + layout.main.width
                && y >= layout.main.y
                && y < layout.main.y + layout.main.height
            {
                if state.question_scroll > 0 {
                    state.question_scroll -= 1;
                }
            }
        }
        MouseEventKind::ScrollDown => {
            let x = mouse.column;
            let y = mouse.row;

            if x >= layout.sidebar.x
                && x < layout.sidebar.x + layout.sidebar.width
                && y >= layout.sidebar.y
                && y < layout.sidebar.y + layout.sidebar.height
            {
                let total = state.quiz.questions.len();
                if state.current_question + 1 < total {
                    state.navigate_to(state.current_question + 1);
                }
            } else if x >= layout.main.x
                && x < layout.main.x + layout.main.width
                && y >= layout.main.y
                && y < layout.main.y + layout.main.height
            {
                state.question_scroll += 1;
            }
        }
        _ => {}
    }

    Ok(())
}

fn scrollbar_navigate(state: &mut AppState, y: u16, track_start: u16, track_end: u16) {
    let total = state.quiz.questions.len();
    if total == 0 {
        return;
    }
    let track_len = track_end.saturating_sub(track_start) as usize;
    if track_len == 0 {
        return;
    }
    let rel = y.saturating_sub(track_start) as usize;
    let target = if track_len <= 1 {
        0
    } else {
        (rel * total.saturating_sub(1)) / (track_len - 1)
    };
    let target = target.min(total.saturating_sub(1));
    state.navigate_to(target);
}

fn disable_raw_mode_safe() {
    disable_raw_mode().ok();
}
