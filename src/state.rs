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
    DoneRequiresAnswer,
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

#[derive(Debug, Clone, PartialEq)]
pub enum MainFocus {
    Answer,
    Hint,
    DoneButton,
    FlagButton,
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
    pub main_focus: MainFocus,
    pub dragging_scrollbar: bool,
    pub done_marks: HashMap<u32, bool>,
    pub status_filter: [bool; 5],
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
            main_focus: MainFocus::Answer,
            dragging_scrollbar: false,
            done_marks: HashMap::new(),
            status_filter: [true; 5],
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
        // For the current Short/Long question, use live text_input length
        let is_current_text = self.current_question()
            .filter(|q| q.number == qnum)
            .map_or(false, |q| matches!(q.kind, QuestionKind::Short | QuestionKind::Long));
        let current_text_empty = is_current_text && self.text_input.is_empty();

        // Done is invalid when the current text field is empty
        if !current_text_empty && self.done_marks.get(&qnum).copied().unwrap_or(false) {
            return QuestionStatus::Done;
        }
        if self.flags.get(&qnum).copied().unwrap_or(false) {
            return QuestionStatus::Flagged;
        }
        if is_current_text {
            if !self.text_input.is_empty() {
                return QuestionStatus::Answered;
            }
        } else if self.answers.contains_key(&qnum) {
            return QuestionStatus::Answered;
        }
        if self.visited.get(&qnum).copied().unwrap_or(false) {
            return QuestionStatus::NotAnswered;
        }
        QuestionStatus::Unread
    }

    pub fn status_counts(&self) -> StatusCounts {
        let mut counts = StatusCounts::default();
        for q in &self.quiz.questions {
            match self.question_status(q.number) {
                QuestionStatus::Unread => counts.unread += 1,
                QuestionStatus::NotAnswered => counts.not_answered += 1,
                QuestionStatus::Answered => counts.answered += 1,
                QuestionStatus::Done => counts.done += 1,
                QuestionStatus::Flagged => counts.flagged += 1,
            }
        }
        counts
    }

    /// Toggle done mark. Returns false if marking done but no answer exists.
    pub fn toggle_done(&mut self) -> bool {
        let qnum = self.current_question_number();
        let currently_done = self.done_marks.get(&qnum).copied().unwrap_or(false);
        if currently_done {
            self.done_marks.insert(qnum, false);
            true
        } else {
            // For current Short/Long, check live text_input instead of answers map
            let has_answer = {
                let is_current_text = self.current_question()
                    .map_or(false, |q| matches!(q.kind, QuestionKind::Short | QuestionKind::Long));
                if is_current_text {
                    !self.text_input.is_empty()
                } else {
                    self.answers.contains_key(&qnum)
                }
            };
            if !has_answer {
                return false;
            }
            // Save text so the answer is persisted before marking done
            self.save_current_text_input();
            self.done_marks.insert(qnum, true);
            // Mutually exclusive: clear flag
            self.flags.insert(qnum, false);
            true
        }
    }

    pub fn toggle_flag(&mut self) {
        let qnum = self.current_question_number();
        let current = self.flags.get(&qnum).copied().unwrap_or(false);
        if current {
            self.flags.insert(qnum, false);
        } else {
            self.flags.insert(qnum, true);
            // Mutually exclusive: clear done
            self.done_marks.insert(qnum, false);
        }
    }

    pub fn is_done(&self, qnum: u32) -> bool {
        if !self.done_marks.get(&qnum).copied().unwrap_or(false) {
            return false;
        }
        // For the current Short/Long question, done is invalid when text is empty
        let is_current_text = self.current_question()
            .filter(|q| q.number == qnum)
            .map_or(false, |q| matches!(q.kind, QuestionKind::Short | QuestionKind::Long));
        if is_current_text && self.text_input.is_empty() {
            return false;
        }
        true
    }

    pub fn is_flagged(&self, qnum: u32) -> bool {
        self.flags.get(&qnum).copied().unwrap_or(false)
    }

    pub fn is_status_visible(&self, status: QuestionStatus) -> bool {
        match status {
            QuestionStatus::Done => self.status_filter[0],
            QuestionStatus::Answered => self.status_filter[1],
            QuestionStatus::Flagged => self.status_filter[2],
            QuestionStatus::NotAnswered => self.status_filter[3],
            QuestionStatus::Unread => self.status_filter[4],
        }
    }

    pub fn toggle_status_filter(&mut self, idx: usize) {
        if idx < 5 {
            self.status_filter[idx] = !self.status_filter[idx];
        }
    }

    /// Returns indices into quiz.questions for questions whose status passes the filter.
    pub fn filtered_questions(&self) -> Vec<usize> {
        self.quiz
            .questions
            .iter()
            .enumerate()
            .filter(|(_, q)| self.is_status_visible(self.question_status(q.number)))
            .map(|(i, _)| i)
            .collect()
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
            self.main_focus = MainFocus::Answer;
            self.update_input_mode();
        }
    }

    pub fn cycle_main_focus(&mut self) {
        let has_unrevealed_hints = self.current_question().map_or(false, |q| {
            let qnum = q.number;
            let revealed = self.hints_revealed.get(&qnum).copied().unwrap_or(0);
            q.hints.len() > 0 && revealed < q.hints.len()
        });

        self.main_focus = match self.main_focus {
            MainFocus::Answer => {
                // Leaving Answer: save text input, switch to Navigation
                self.save_current_text_input();
                self.input_mode = InputMode::Navigation;
                if has_unrevealed_hints {
                    MainFocus::Hint
                } else {
                    MainFocus::DoneButton
                }
            }
            MainFocus::Hint => MainFocus::DoneButton,
            MainFocus::DoneButton => MainFocus::FlagButton,
            MainFocus::FlagButton => {
                // Entering Answer: restore appropriate input mode
                let focus = MainFocus::Answer;
                self.update_input_mode();
                focus
            }
        };
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
                    } else {
                        self.answers.remove(&q.number);
                        self.done_marks.insert(q.number, false);
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
                    } else {
                        self.answers.remove(&q.number);
                        self.done_marks.insert(q.number, false);
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
                QuestionKind::Long => {
                    self.input_mode = InputMode::TextInput;
                }
                QuestionKind::File(_) => {
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
    NotAnswered,
    Answered,
    Done,
    Flagged,
}

#[derive(Debug, Default)]
pub struct StatusCounts {
    pub unread: usize,
    pub not_answered: usize,
    pub answered: usize,
    pub done: usize,
    pub flagged: usize,
}
