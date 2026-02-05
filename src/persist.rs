use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::model::{AckData, Answer};
use crate::state::AppState;

pub fn state_dir_for(canonical_path: &Path) -> PathBuf {
    if let Ok(override_dir) = std::env::var("TERMQUIZ_STATE") {
        return PathBuf::from(override_dir);
    }

    let hash = {
        let mut hasher = Sha256::new();
        hasher.update(canonical_path.to_string_lossy().as_bytes());
        let result = hasher.finalize();
        hex_encode(&result[..4])
    };

    let base = dirs_state_base();
    base.join("termquiz").join(hash)
}

fn dirs_state_base() -> PathBuf {
    if let Ok(state_home) = std::env::var("XDG_STATE_HOME") {
        PathBuf::from(state_home)
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".local").join("state")
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn compute_file_hash(path: &Path) -> Result<String, String> {
    let content =
        fs::read(path).map_err(|e| format!("Cannot read file {}: {}", path.display(), e))?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    let result = hasher.finalize();
    Ok(format!("sha256:{}", hex_encode(&result)))
}

pub fn compute_str_hash(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    let result = hasher.finalize();
    format!("sha256:{}", hex_encode(&result))
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct SessionData {
    pub started_at: Option<String>,
    pub current_question: usize,
    pub quiz_file_hash: String,
    #[serde(default)]
    pub acknowledgment: Option<AckData>,
}

pub fn save_state(state: &AppState, state_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(state_dir)
        .map_err(|e| format!("Cannot create state dir: {}", e))?;

    // Save session
    let session = SessionData {
        started_at: state.started_at.clone(),
        current_question: state.current_question,
        quiz_file_hash: state.quiz.quiz_hash.clone(),
        acknowledgment: state.ack_data.clone(),
    };

    let session_toml =
        toml::to_string_pretty(&session).map_err(|e| format!("Cannot serialize session: {}", e))?;
    atomic_write(&state_dir.join("session.toml"), &session_toml)?;

    // Save answers
    let answers_toml = serialize_answers(&state.answers)?;
    atomic_write(&state_dir.join("answers.toml"), &answers_toml)?;

    Ok(())
}

pub fn load_state(state: &mut AppState, state_dir: &Path) -> Result<bool, String> {
    let session_path = state_dir.join("session.toml");
    if !session_path.exists() {
        return Ok(false);
    }

    let session_str = fs::read_to_string(&session_path)
        .map_err(|e| format!("Cannot read session: {}", e))?;
    let session: SessionData = toml::from_str(&session_str)
        .map_err(|e| format!("Corrupt session.toml: {} (use --clear to reset)", e))?;

    // Verify hash matches
    if session.quiz_file_hash != state.quiz.quiz_hash {
        return Err("Quiz file has changed since last session. Use --clear to reset.".to_string());
    }

    state.started_at = session.started_at;
    state.ack_data = session.acknowledgment;

    // Navigate to saved question
    if session.current_question < state.quiz.questions.len() {
        state.current_question = session.current_question;
    }

    // Load answers
    let answers_path = state_dir.join("answers.toml");
    if answers_path.exists() {
        let answers_str = fs::read_to_string(&answers_path)
            .map_err(|e| format!("Cannot read answers: {}", e))?;
        let answers = deserialize_answers(&answers_str)?;
        state.answers = answers;

        // Mark all answered questions as visited
        for &qnum in state.answers.keys() {
            state.visited.insert(qnum, true);
        }
    }

    Ok(true)
}

pub fn clear_state(state_dir: &Path) -> Result<(), String> {
    if state_dir.exists() {
        fs::remove_dir_all(state_dir)
            .map_err(|e| format!("Cannot clear state: {}", e))?;
    }
    Ok(())
}

fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, content).map_err(|e| format!("Cannot write {}: {}", tmp.display(), e))?;
    fs::rename(&tmp, path).map_err(|e| format!("Cannot rename: {}", e))?;
    Ok(())
}

fn serialize_answers(answers: &HashMap<u32, Answer>) -> Result<String, String> {
    // Build a BTreeMap for sorted output
    let mut map = std::collections::BTreeMap::new();
    for (qnum, answer) in answers {
        map.insert(format!("q{}", qnum), answer.clone());
    }
    toml::to_string_pretty(&map).map_err(|e| format!("Cannot serialize answers: {}", e))
}

fn deserialize_answers(s: &str) -> Result<HashMap<u32, Answer>, String> {
    let map: std::collections::BTreeMap<String, Answer> =
        toml::from_str(s).map_err(|e| format!("Corrupt answers.toml: {} (use --clear to reset)", e))?;

    let mut answers = HashMap::new();
    for (key, answer) in map {
        if let Some(num_str) = key.strip_prefix('q') {
            if let Ok(num) = num_str.parse::<u32>() {
                answers.insert(num, answer);
            }
        }
    }
    Ok(answers)
}

pub fn export_answers(state: &AppState, path: &str) -> Result<(), String> {
    let answers_toml = serialize_answers(&state.answers)?;
    fs::write(path, &answers_toml).map_err(|e| format!("Cannot export: {}", e))?;
    Ok(())
}

pub fn print_status(state: &AppState) {
    let counts = state.status_counts();
    let total = state.quiz.questions.len();
    println!("Quiz: {}", state.quiz.title);
    println!("Questions: {}", total);
    println!(
        "  Done: {}, Partial: {}, Empty: {}, Flagged: {}, Unread: {}",
        counts.done, counts.partial, counts.empty, counts.flagged, counts.unread
    );
    if let Some(ref started) = state.started_at {
        println!("Started: {}", started);
    }
}
