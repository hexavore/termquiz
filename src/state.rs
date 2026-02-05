use std::collections::HashMap;

use crate::model::*;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Waiting,
    Preamble,
    Acknowledgment,
    Working,
    Closed,
    AlreadySubmitted,
    Pushing,
    PushRetrying,
    SaveLocal,
    Done,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Dialog {
    ConfirmSubmit,
    ConfirmQuit,
    ConfirmHint,
    ConfirmDeleteFile(usize),
    TwoMinuteWarning,
    Help,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Navigation,
    ChoiceSelect,
    TextInput,
    AckNameInput,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AckFocus {
    Name,
    Checkbox,
    Ok,
    Cancel,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActivePanel {
    Sidebar,
    Main,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub screen: Screen,
    pub quiz: Quiz,
    pub current_question: usize,
    pub answers: HashMap<u32, Answer>,
    pub flags: HashMap<u32, bool>,
    pub visited: HashMap<u32, bool>,
    pub hints_revealed: HashMap<u32, usize>,
    pub input_mode: InputMode,
    pub dialog_stack: Vec<Dialog>,
    pub choice_cursor: usize,
    pub text_input: String,
    pub text_cursor: usize,
    pub remaining_seconds: Option<i64>,
    pub started_at: Option<String>,
    pub submitted_at: Option<String>,
    pub repo_dir: std::path::PathBuf,
    pub should_quit: bool,
    pub ack_data: Option<AckData>,
    pub ack_name: String,
    pub ack_checkbox: bool,
    pub ack_focus: AckFocus,
    pub push_attempt: u32,
    pub push_retry_secs: u32,
    pub push_elapsed_secs: u32,
    pub push_error: String,
    pub sidebar_scroll: usize,
    pub question_scroll: usize,
    pub file_cursor: usize,
    pub active_panel: ActivePanel,
    pub dragging_scrollbar: bool,
}

impl AppState {
    pub fn new(quiz: Quiz, repo_dir: std::path::PathBuf) -> Self {
        Self {
            screen: Screen::Working,
            quiz,
            current_question: 0,
            answers: HashMap::new(),
            flags: HashMap::new(),
            visited: HashMap::new(),
            hints_revealed: HashMap::new(),
            input_mode: InputMode::Navigation,
            dialog_stack: Vec::new(),
            choice_cursor: 0,
            text_input: String::new(),
            text_cursor: 0,
            remaining_seconds: None,
            started_at: None,
            submitted_at: None,
            repo_dir,
            should_quit: false,
            ack_data: None,
            ack_name: String::new(),
            ack_checkbox: false,
            ack_focus: AckFocus::Name,
            push_attempt: 0,
            push_retry_secs: 0,
            push_elapsed_secs: 0,
            push_error: String::new(),
            sidebar_scroll: 0,
            question_scroll: 0,
            file_cursor: 0,
            active_panel: ActivePanel::Main,
            dragging_scrollbar: false,
        }
    }

    pub fn current_question(&self) -> Option<&Question> {
        self.quiz.questions.get(self.current_question)
    }

    pub fn current_question_number(&self) -> u32 {
        self.current_question()
            .map(|q| q.number)
            .unwrap_or(0)
    }

    pub fn question_status(&self, qnum: u32) -> QuestionStatus {
        if self.flags.get(&qnum).copied().unwrap_or(false) {
            return QuestionStatus::Flagged;
        }

        if let Some(answer) = self.answers.get(&qnum) {
            if is_answer_complete(answer) {
                QuestionStatus::Done
            } else {
                QuestionStatus::Partial
            }
        } else if self.visited.get(&qnum).copied().unwrap_or(false) {
            QuestionStatus::Empty
        } else {
            QuestionStatus::Unread
        }
    }

    pub fn status_counts(&self) -> StatusCounts {
        let mut counts = StatusCounts::default();
        for q in &self.quiz.questions {
            match self.question_status(q.number) {
                QuestionStatus::Unread => counts.unread += 1,
                QuestionStatus::Empty => counts.empty += 1,
                QuestionStatus::Partial => counts.partial += 1,
                QuestionStatus::Done => counts.done += 1,
                QuestionStatus::Flagged => counts.flagged += 1,
            }
        }
        counts
    }

    pub fn navigate_to(&mut self, idx: usize) {
        if idx < self.quiz.questions.len() {
            // Save current text input
            self.save_current_text_input();
            self.current_question = idx;
            let qnum = self.quiz.questions[idx].number;
            self.visited.insert(qnum, true);
            // Load answer text if exists
            self.load_text_input_for_current();
            self.choice_cursor = 0;
            self.question_scroll = 0;
            self.file_cursor = 0;
            self.update_input_mode();
        }
    }

    pub fn save_current_text_input(&mut self) {
        if let Some(q) = self.current_question().cloned() {
            match &q.kind {
                QuestionKind::Short => {
                    if !self.text_input.is_empty() {
                        self.answers.insert(
                            q.number,
                            Answer {
                                answer_type: "short".to_string(),
                                selected: None,
                                text: Some(self.text_input.clone()),
                                files: None,
                            },
                        );
                    }
                }
                QuestionKind::Long => {
                    if !self.text_input.is_empty() {
                        self.answers.insert(
                            q.number,
                            Answer {
                                answer_type: "long".to_string(),
                                selected: None,
                                text: Some(self.text_input.clone()),
                                files: None,
                            },
                        );
                    }
                }
                _ => {}
            }
        }
    }

    pub fn load_text_input_for_current(&mut self) {
        if let Some(q) = self.current_question() {
            let qnum = q.number;
            if let Some(answer) = self.answers.get(&qnum) {
                if let Some(text) = &answer.text {
                    self.text_input = text.clone();
                    self.text_cursor = self.text_input.len();
                    return;
                }
            }
        }
        self.text_input.clear();
        self.text_cursor = 0;
    }

    fn update_input_mode(&mut self) {
        if let Some(q) = self.current_question() {
            match &q.kind {
                QuestionKind::SingleChoice(_) | QuestionKind::MultiChoice(_) => {
                    self.input_mode = InputMode::ChoiceSelect;
                }
                QuestionKind::Short => {
                    self.input_mode = InputMode::TextInput;
                }
                QuestionKind::Long | QuestionKind::File(_) => {
                    self.input_mode = InputMode::Navigation;
                }
            }
        }
    }

    pub fn select_single_choice(&mut self, idx: usize) {
        if let Some(q) = self.current_question().cloned() {
            if let QuestionKind::SingleChoice(choices) = &q.kind {
                if idx < choices.len() {
                    let label = choices[idx].label.to_string();
                    self.answers.insert(
                        q.number,
                        Answer {
                            answer_type: "single".to_string(),
                            selected: Some(vec![label]),
                            text: None,
                            files: None,
                        },
                    );
                }
            }
        }
    }

    pub fn toggle_multi_choice(&mut self, idx: usize) {
        if let Some(q) = self.current_question().cloned() {
            if let QuestionKind::MultiChoice(choices) = &q.kind {
                if idx < choices.len() {
                    let label = choices[idx].label.to_string();
                    let mut selected = if let Some(existing) = self.answers.get(&q.number) {
                        existing.selected.clone().unwrap_or_default()
                    } else {
                        Vec::new()
                    };

                    if selected.contains(&label) {
                        selected.retain(|s| s != &label);
                    } else {
                        selected.push(label);
                    }

                    self.answers.insert(
                        q.number,
                        Answer {
                            answer_type: "multi".to_string(),
                            selected: Some(selected),
                            text: None,
                            files: None,
                        },
                    );
                }
            }
        }
    }

    pub fn is_choice_selected(&self, qnum: u32, label: char) -> bool {
        if let Some(answer) = self.answers.get(&qnum) {
            if let Some(selected) = &answer.selected {
                return selected.contains(&label.to_string());
            }
        }
        false
    }

    pub fn get_file_list(&self, qnum: u32) -> Vec<String> {
        if let Some(answer) = self.answers.get(&qnum) {
            if let Some(files) = &answer.files {
                return files.clone();
            }
        }
        Vec::new()
    }

    pub fn add_file(&mut self, qnum: u32, file_path: String) {
        let existing = self.answers.entry(qnum).or_insert_with(|| Answer {
            answer_type: "file".to_string(),
            selected: None,
            text: None,
            files: Some(Vec::new()),
        });
        if let Some(files) = &mut existing.files {
            files.push(file_path);
        }
    }

    pub fn remove_file(&mut self, qnum: u32, idx: usize) {
        if let Some(answer) = self.answers.get_mut(&qnum) {
            if let Some(files) = &mut answer.files {
                if idx < files.len() {
                    files.remove(idx);
                }
            }
        }
    }

    pub fn has_dialog(&self) -> bool {
        !self.dialog_stack.is_empty()
    }

    pub fn top_dialog(&self) -> Option<&Dialog> {
        self.dialog_stack.last()
    }

    pub fn push_dialog(&mut self, dialog: Dialog) {
        self.dialog_stack.push(dialog);
    }

    pub fn pop_dialog(&mut self) -> Option<Dialog> {
        self.dialog_stack.pop()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuestionStatus {
    Unread,
    Empty,
    Partial,
    Done,
    Flagged,
}

#[derive(Debug, Default)]
pub struct StatusCounts {
    pub unread: usize,
    pub empty: usize,
    pub partial: usize,
    pub done: usize,
    pub flagged: usize,
}

fn is_answer_complete(answer: &Answer) -> bool {
    match answer.answer_type.as_str() {
        "single" => answer
            .selected
            .as_ref()
            .map(|s| !s.is_empty())
            .unwrap_or(false),
        "multi" => answer
            .selected
            .as_ref()
            .map(|s| !s.is_empty())
            .unwrap_or(false),
        "short" => answer
            .text
            .as_ref()
            .map(|t| !t.trim().is_empty())
            .unwrap_or(false),
        "long" => answer
            .text
            .as_ref()
            .map(|t| !t.trim().is_empty())
            .unwrap_or(false),
        "file" => answer
            .files
            .as_ref()
            .map(|f| !f.is_empty())
            .unwrap_or(false),
        _ => false,
    }
}
