use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::model::AckData;
use crate::state::AppState;
use crate::submit;

pub fn save_state(state: &AppState) -> Result<(), String> {
    let response_dir = state.repo_dir.join("response");
    fs::create_dir_all(&response_dir)
        .map_err(|e| format!("Cannot create response dir: {}", e))?;

    let yaml = submit::build_answers_yaml(state);
    atomic_write(&response_dir.join("answers.yaml"), &yaml)?;

    Ok(())
}

pub fn load_state(state: &mut AppState) -> Result<bool, String> {
    let yaml_path = state.repo_dir.join("response").join("answers.yaml");
    if !yaml_path.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(&yaml_path)
        .map_err(|e| format!("Cannot read answers.yaml: {}", e))?;

    let doc: serde_yaml::Value = serde_yaml::from_str(&content)
        .map_err(|e| format!("Corrupt answers.yaml: {} (use --clear to reset)", e))?;

    // Verify quiz hash
    if let Some(hash) = doc["session"]["quiz_file_hash"].as_str() {
        if hash != state.quiz.quiz_hash {
            return Err("Quiz file has changed since last session. Use --clear to reset.".to_string());
        }
    }

    // Restore session metadata
    if let Some(v) = doc["session"]["current_question"].as_u64() {
        if (v as usize) < state.quiz.questions.len() {
            state.current_question = v as usize;
        }
    }
    if let Some(v) = doc["quiz"]["submitted_at"].as_str() {
        if v != "unknown" {
            state.submitted_at = Some(v.to_string());
        }
    }
    if let Some(v) = doc["session"]["started_at"].as_str() {
        state.started_at = Some(v.to_string());
    }

    // Restore acknowledgment
    if let Some(ack) = doc["session"]["acknowledgment"].as_mapping() {
        let name = ack.get(serde_yaml::Value::String("name".into()))
            .and_then(|v| v.as_str()).unwrap_or("").to_string();
        let agreed_at = ack.get(serde_yaml::Value::String("agreed_at".into()))
            .and_then(|v| v.as_str()).unwrap_or("").to_string();
        let text_hash = ack.get(serde_yaml::Value::String("text_hash".into()))
            .and_then(|v| v.as_str()).unwrap_or("").to_string();
        if !name.is_empty() {
            state.ack_data = Some(AckData { name, agreed_at, text_hash });
        }
    }

    // Restore per-question data
    if let Some(questions) = doc["questions"].as_sequence() {
        for q_val in questions {
            let number = match q_val["number"].as_u64() {
                Some(n) => n as u32,
                None => continue,
            };
            let qtype = q_val["type"].as_str().unwrap_or("");

            // Restore answer
            let answer_val = &q_val["answer"];
            if !answer_val.is_null() {
                let answer = match qtype {
                    "single" => {
                        if let Some(label) = answer_val.as_str() {
                            Some(crate::model::Answer {
                                answer_type: "single".to_string(),
                                selected: Some(vec![label.to_string()]),
                                text: None,
                                files: None,
                            })
                        } else { None }
                    }
                    "multi" => {
                        if let Some(seq) = answer_val.as_sequence() {
                            let labels: Vec<String> = seq.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect();
                            Some(crate::model::Answer {
                                answer_type: "multi".to_string(),
                                selected: Some(labels),
                                text: None,
                                files: None,
                            })
                        } else { None }
                    }
                    "short" => {
                        if let Some(text) = answer_val.as_str() {
                            Some(crate::model::Answer {
                                answer_type: "short".to_string(),
                                selected: None,
                                text: Some(text.to_string()),
                                files: None,
                            })
                        } else { None }
                    }
                    "long" => {
                        if let Some(text) = answer_val.as_str() {
                            Some(crate::model::Answer {
                                answer_type: "long".to_string(),
                                selected: None,
                                text: Some(text.to_string()),
                                files: None,
                            })
                        } else { None }
                    }
                    "file" => {
                        if let Some(seq) = answer_val.as_sequence() {
                            let files: Vec<String> = seq.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect();
                            Some(crate::model::Answer {
                                answer_type: "file".to_string(),
                                selected: None,
                                text: None,
                                files: Some(files),
                            })
                        } else { None }
                    }
                    _ => None,
                };
                if let Some(a) = answer {
                    state.answers.insert(number, a);
                    state.visited.insert(number, true);
                }
            }

            // Restore done/flagged
            if q_val["done"].as_bool().unwrap_or(false) {
                state.done_marks.insert(number, true);
            }
            if q_val["flagged"].as_bool().unwrap_or(false) {
                state.flags.insert(number, true);
            }

            // Restore hint_used
            if q_val["hint_used"].as_bool().unwrap_or(false) {
                state.hints_revealed.insert(number, 1);
            }
        }
    }

    Ok(true)
}

pub fn clear_state(repo_dir: &Path) -> Result<(), String> {
    let response_dir = repo_dir.join("response");
    if response_dir.exists() {
        fs::remove_dir_all(&response_dir)
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

pub fn export_answers(state: &AppState, path: &str) -> Result<(), String> {
    let yaml = submit::build_answers_yaml(state);
    fs::write(path, &yaml).map_err(|e| format!("Cannot export: {}", e))?;
    Ok(())
}

pub fn print_status(state: &AppState) {
    let counts = state.status_counts();
    let total = state.quiz.questions.len();
    println!("Quiz: {}", state.quiz.title);
    println!("Questions: {}", total);
    println!(
        "  Done: {}, Answered: {}, Not answered: {}, Flagged: {}, Unread: {}",
        counts.done, counts.answered, counts.not_answered, counts.flagged, counts.unread
    );
    if let Some(ref started) = state.started_at {
        println!("Started: {}", started);
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
