use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::model::Answer;
use crate::state::AppState;

pub fn build_response(state: &AppState, repo_dir: &Path) -> Result<(), String> {
    let response_dir = repo_dir.join("response");
    fs::create_dir_all(&response_dir)
        .map_err(|e| format!("Cannot create response dir: {}", e))?;

    // Build meta.toml
    let meta = build_meta_toml(state);
    fs::write(response_dir.join("meta.toml"), &meta)
        .map_err(|e| format!("Cannot write meta.toml: {}", e))?;

    // Build answers.toml
    let answers = build_answers_toml(&state.answers)?;
    fs::write(response_dir.join("answers.toml"), &answers)
        .map_err(|e| format!("Cannot write answers.toml: {}", e))?;

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

fn build_meta_toml(state: &AppState) -> String {
    let mut meta = String::new();
    meta.push_str(&format!("quiz_file = {:?}\n", state.quiz.quiz_file));
    meta.push_str(&format!("quiz_hash = {:?}\n", state.quiz.quiz_hash));
    meta.push_str(&format!(
        "started_at = {:?}\n",
        state.started_at.as_deref().unwrap_or("unknown")
    ));
    meta.push_str(&format!(
        "submitted_at = {:?}\n",
        state.submitted_at.as_deref().unwrap_or("unknown")
    ));
    meta.push_str(&format!("termquiz_version = {:?}\n", env!("CARGO_PKG_VERSION")));

    if let Some(ref ack) = state.ack_data {
        meta.push_str("\n[acknowledgment]\n");
        meta.push_str(&format!("name = {:?}\n", ack.name));
        meta.push_str(&format!("agreed_at = {:?}\n", ack.agreed_at));
        meta.push_str(&format!("text_hash = {:?}\n", ack.text_hash));
    }

    // hints used
    let mut hints_used: Vec<(u32, usize)> = state
        .hints_revealed
        .iter()
        .filter(|(_, &count)| count > 0)
        .map(|(&q, &c)| (q, c))
        .collect();
    hints_used.sort_by_key(|(q, _)| *q);

    if !hints_used.is_empty() {
        meta.push_str("\n[hints_used]\n");
        for (q, count) in hints_used {
            meta.push_str(&format!("q{} = {}\n", q, count));
        }
    }

    meta
}

fn build_answers_toml(answers: &HashMap<u32, Answer>) -> Result<String, String> {
    let mut map = std::collections::BTreeMap::new();
    for (qnum, answer) in answers {
        map.insert(format!("q{}", qnum), answer.clone());
    }
    toml::to_string_pretty(&map).map_err(|e| format!("Cannot serialize answers: {}", e))
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
