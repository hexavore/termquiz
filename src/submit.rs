use std::fs;
use std::path::Path;

use crate::model::QuestionKind;
use crate::state::AppState;

/// Build response directory: copy file attachments.
/// The answers.yaml is already written by persist::save_state.
pub fn build_response(state: &AppState, repo_dir: &Path) -> Result<(), String> {
    let response_dir = repo_dir.join("response");
    fs::create_dir_all(&response_dir)
        .map_err(|e| format!("Cannot create response dir: {}", e))?;

    // Copy file attachments
    let files_dir = response_dir.join("files");
    for (qnum, answer) in &state.answers {
        if let Some(file_list) = &answer.files {
            let q_dir = files_dir.join(format!("q{}", qnum));
            fs::create_dir_all(&q_dir)
                .map_err(|e| format!("Cannot create files dir: {}", e))?;

            for file_path in file_list {
                let src = Path::new(file_path);
                if src.exists() {
                    let filename = src
                        .file_name()
                        .ok_or_else(|| "Invalid file name".to_string())?;
                    let dest = q_dir.join(filename);
                    fs::copy(src, &dest)
                        .map_err(|e| format!("Cannot copy file: {}", e))?;
                }
            }
        }
    }

    Ok(())
}

pub fn build_answers_yaml(state: &AppState) -> String {
    let mut out = String::new();

    // quiz metadata
    out.push_str("quiz:\n");
    out.push_str(&format!("  title: {:?}\n", state.quiz.title));
    out.push_str(&format!("  source: {:?}\n", state.quiz.quiz_file));
    out.push_str(&format!(
        "  submitted_at: {:?}\n",
        state.submitted_at.as_deref().unwrap_or("unknown")
    ));
    out.push_str(&format!(
        "  duration: {:?}\n",
        compute_duration(&state.started_at, &state.submitted_at)
    ));
    if state.ack_data.is_some() {
        out.push_str("  acknowledged: true\n");
    }

    // session state (for restore on restart)
    out.push_str("\nsession:\n");
    out.push_str(&format!("  current_question: {}\n", state.current_question));
    out.push_str(&format!("  quiz_file_hash: {:?}\n", state.quiz.quiz_hash));
    if let Some(ref started) = state.started_at {
        out.push_str(&format!("  started_at: {:?}\n", started));
    }
    if let Some(ref ack) = state.ack_data {
        out.push_str("  acknowledgment:\n");
        out.push_str(&format!("    name: {:?}\n", ack.name));
        out.push_str(&format!("    agreed_at: {:?}\n", ack.agreed_at));
        out.push_str(&format!("    text_hash: {:?}\n", ack.text_hash));
    }

    // questions
    out.push_str("\nquestions:\n");
    for q in &state.quiz.questions {
        out.push_str(&format!("  - number: {}\n", q.number));
        out.push_str(&format!("    title: {:?}\n", q.title));

        let answer = state.answers.get(&q.number);
        let hint_used = state.hints_revealed.get(&q.number).copied().unwrap_or(0) > 0;
        let done = state.done_marks.get(&q.number).copied().unwrap_or(false);
        let flagged = state.flags.get(&q.number).copied().unwrap_or(false);

        match &q.kind {
            QuestionKind::SingleChoice(choices) => {
                out.push_str("    type: single\n");
                out.push_str("    choices:\n");
                for c in choices {
                    out.push_str(&format!("      {}: {:?}\n", c.label, c.text));
                }
                if hint_used {
                    out.push_str("    hint_used: true\n");
                }
                if done {
                    out.push_str("    done: true\n");
                }
                if flagged {
                    out.push_str("    flagged: true\n");
                }
                match answer.and_then(|a| a.selected.as_ref()) {
                    Some(sel) if !sel.is_empty() => {
                        out.push_str(&format!("    answer: {}\n", sel[0]));
                    }
                    _ => out.push_str("    answer: null\n"),
                }
            }
            QuestionKind::MultiChoice(choices) => {
                out.push_str("    type: multi\n");
                out.push_str("    choices:\n");
                for c in choices {
                    out.push_str(&format!("      {}: {:?}\n", c.label, c.text));
                }
                if hint_used {
                    out.push_str("    hint_used: true\n");
                }
                if done {
                    out.push_str("    done: true\n");
                }
                if flagged {
                    out.push_str("    flagged: true\n");
                }
                match answer.and_then(|a| a.selected.as_ref()) {
                    Some(sel) if !sel.is_empty() => {
                        let labels: Vec<&str> = sel.iter().map(|s| s.as_str()).collect();
                        out.push_str(&format!("    answer: [{}]\n", labels.join(", ")));
                    }
                    _ => out.push_str("    answer: null\n"),
                }
            }
            QuestionKind::Short => {
                out.push_str("    type: short\n");
                if hint_used {
                    out.push_str("    hint_used: true\n");
                }
                if done {
                    out.push_str("    done: true\n");
                }
                if flagged {
                    out.push_str("    flagged: true\n");
                }
                match answer.and_then(|a| a.text.as_ref()) {
                    Some(text) => {
                        out.push_str(&format!("    answer: {:?}\n", text));
                    }
                    None => out.push_str("    answer: null\n"),
                }
            }
            QuestionKind::Long => {
                out.push_str("    type: long\n");
                if hint_used {
                    out.push_str("    hint_used: true\n");
                }
                if done {
                    out.push_str("    done: true\n");
                }
                if flagged {
                    out.push_str("    flagged: true\n");
                }
                match answer.and_then(|a| a.text.as_ref()) {
                    Some(text) => {
                        out.push_str("    answer: |\n");
                        for line in text.lines() {
                            out.push_str(&format!("      {}\n", line));
                        }
                    }
                    None => out.push_str("    answer: null\n"),
                }
            }
            QuestionKind::File(_) => {
                out.push_str("    type: file\n");
                if hint_used {
                    out.push_str("    hint_used: true\n");
                }
                if done {
                    out.push_str("    done: true\n");
                }
                if flagged {
                    out.push_str("    flagged: true\n");
                }
                match answer.and_then(|a| a.files.as_ref()) {
                    Some(files) if !files.is_empty() => {
                        out.push_str("    answer:\n");
                        for f in files {
                            let filename = Path::new(f)
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| f.clone());
                            out.push_str(&format!(
                                "      - files/q{}/{}\n",
                                q.number, filename
                            ));
                        }
                    }
                    _ => out.push_str("    answer: null\n"),
                }
            }
        }
    }

    out
}

fn compute_duration(started: &Option<String>, submitted: &Option<String>) -> String {
    if let (Some(s), Some(e)) = (started, submitted) {
        if let (Ok(start), Ok(end)) = (
            chrono::DateTime::parse_from_rfc3339(s),
            chrono::DateTime::parse_from_rfc3339(e),
        ) {
            let secs = (end - start).num_seconds().max(0);
            let h = secs / 3600;
            let m = (secs % 3600) / 60;
            let s = secs % 60;
            return format!("{:02}:{:02}:{:02}", h, m, s);
        }
    }
    "unknown".to_string()
}

pub fn build_commit_message(state: &AppState) -> String {
    let counts = state.status_counts();
    let total = state.quiz.questions.len();
    format!(
        "termquiz: submit {}\n\nStarted: {}\nSubmitted: {}\nQuestions: {} ({} done, {} answered, {} flagged, {} not answered)",
        state.quiz.quiz_file,
        state.started_at.as_deref().unwrap_or("unknown"),
        state.submitted_at.as_deref().unwrap_or("unknown"),
        total,
        counts.done,
        counts.answered,
        counts.flagged,
        counts.not_answered + counts.unread,
    )
}
