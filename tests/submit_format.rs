use std::fs;
use std::path::PathBuf;

#[test]
fn test_build_response() {
    let content = fs::read_to_string("fixtures/sample_quiz.md").expect("Cannot read fixture");
    let quiz =
        termquiz::parser::parse_quiz(&content, "sample_quiz.md", "sha256:abc123").unwrap();

    let tmp_dir = std::env::temp_dir().join("termquiz_test_submit");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let mut state = termquiz::state::AppState::new(quiz, tmp_dir.clone());
    state.started_at = Some("2025-01-02T10:01:23-05:00".to_string());
    state.submitted_at = Some("2025-01-02T11:23:45-05:00".to_string());

    // Add some answers
    state.answers.insert(
        1,
        termquiz::model::Answer {
            answer_type: "single".to_string(),
            selected: Some(vec!["b".to_string()]),
            text: None,
            files: None,
        },
    );
    state.answers.insert(
        3,
        termquiz::model::Answer {
            answer_type: "short".to_string(),
            selected: None,
            text: Some("-i".to_string()),
            files: None,
        },
    );

    // save_state writes response/answers.yaml
    termquiz::persist::save_state(&state).unwrap();

    // Verify response directory has answers.yaml
    assert!(tmp_dir.join("response/answers.yaml").exists());

    let yaml = fs::read_to_string(tmp_dir.join("response/answers.yaml")).unwrap();

    // Quiz metadata
    assert!(yaml.contains("quiz:"));
    assert!(yaml.contains("sample_quiz.md"));
    assert!(yaml.contains("submitted_at:"));
    assert!(yaml.contains("duration:"));

    // Session block
    assert!(yaml.contains("session:"));
    assert!(yaml.contains("quiz_file_hash:"));
    assert!(yaml.contains("current_question:"));

    // Questions section
    assert!(yaml.contains("questions:"));

    // Q1 single choice with answer
    assert!(yaml.contains("type: single"));
    assert!(yaml.contains("answer: b"));

    // Q3 short answer
    assert!(yaml.contains("type: short"));
    assert!(yaml.contains("answer: \"-i\""));

    // Unanswered questions have null
    let null_count = yaml.matches("answer: null").count();
    assert!(null_count >= 3, "Expected at least 3 null answers, got {}", null_count);

    // Cleanup
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_commit_message() {
    let content = fs::read_to_string("fixtures/sample_quiz.md").expect("Cannot read fixture");
    let quiz =
        termquiz::parser::parse_quiz(&content, "sample_quiz.md", "sha256:abc123").unwrap();

    let state = termquiz::state::AppState::new(quiz, PathBuf::from("/tmp"));

    let msg = termquiz::submit::build_commit_message(&state);
    assert!(msg.starts_with("termquiz: submit"));
    assert!(msg.contains("Questions: 5"));
}

#[test]
fn test_yaml_structure() {
    let content = fs::read_to_string("fixtures/sample_quiz.md").expect("Cannot read fixture");
    let quiz =
        termquiz::parser::parse_quiz(&content, "sample_quiz.md", "sha256:abc123").unwrap();

    let tmp_dir = std::env::temp_dir().join("termquiz_test_yaml_struct");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    let mut state = termquiz::state::AppState::new(quiz, tmp_dir.clone());
    state.started_at = Some("2025-01-02T10:00:00-05:00".to_string());
    state.submitted_at = Some("2025-01-02T11:22:34-05:00".to_string());
    state.ack_data = Some(termquiz::model::AckData {
        name: "Test Student".to_string(),
        agreed_at: "2025-01-02T10:00:05-05:00".to_string(),
        text_hash: "sha256:abc".to_string(),
    });

    // Single choice answer
    state.answers.insert(1, termquiz::model::Answer {
        answer_type: "single".to_string(),
        selected: Some(vec!["b".to_string()]),
        text: None,
        files: None,
    });

    // Multi choice answer
    state.answers.insert(2, termquiz::model::Answer {
        answer_type: "multi".to_string(),
        selected: Some(vec!["a".to_string(), "b".to_string(), "d".to_string()]),
        text: None,
        files: None,
    });

    // Short answer
    state.answers.insert(3, termquiz::model::Answer {
        answer_type: "short".to_string(),
        selected: None,
        text: Some("-i".to_string()),
        files: None,
    });

    // Long answer
    state.answers.insert(4, termquiz::model::Answer {
        answer_type: "long".to_string(),
        selected: None,
        text: Some("The borrow checker ensures\nsafety at compile time.".to_string()),
        files: None,
    });

    // Hint used on Q4, done on Q1, flagged on Q3
    state.hints_revealed.insert(4, 1);
    state.done_marks.insert(1, true);
    state.flags.insert(3, true);

    let yaml = termquiz::submit::build_answers_yaml(&state);

    // Parse as YAML to validate structure
    let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml)
        .expect("Output must be valid YAML");

    // Verify quiz metadata
    let quiz_node = &parsed["quiz"];
    assert_eq!(quiz_node["acknowledged"], serde_yaml::Value::Bool(true));
    assert_eq!(quiz_node["duration"], serde_yaml::Value::String("01:22:34".to_string()));

    // Verify session block
    let session = &parsed["session"];
    assert_eq!(session["current_question"], serde_yaml::Value::Number(0.into()));
    assert!(session["quiz_file_hash"].as_str().is_some());

    // Verify questions is a sequence of 5
    let questions = parsed["questions"].as_sequence().expect("questions must be a sequence");
    assert_eq!(questions.len(), 5);

    // Q1: single choice, done
    assert_eq!(questions[0]["type"], serde_yaml::Value::String("single".to_string()));
    assert_eq!(questions[0]["answer"], serde_yaml::Value::String("b".to_string()));
    assert_eq!(questions[0]["done"], serde_yaml::Value::Bool(true));
    assert!(questions[0]["choices"].as_mapping().is_some());

    // Q2: multi choice
    assert_eq!(questions[1]["type"], serde_yaml::Value::String("multi".to_string()));
    let q2_answer = questions[1]["answer"].as_sequence().expect("multi answer must be a list");
    assert_eq!(q2_answer.len(), 3);

    // Q3: short, flagged
    assert_eq!(questions[2]["type"], serde_yaml::Value::String("short".to_string()));
    assert_eq!(questions[2]["answer"], serde_yaml::Value::String("-i".to_string()));
    assert_eq!(questions[2]["flagged"], serde_yaml::Value::Bool(true));

    // Q4: long with hint_used
    assert_eq!(questions[3]["type"], serde_yaml::Value::String("long".to_string()));
    assert_eq!(questions[3]["hint_used"], serde_yaml::Value::Bool(true));
    let q4_text = questions[3]["answer"].as_str().expect("long answer must be a string");
    assert!(q4_text.contains("borrow checker"));

    // Q5: file, unanswered
    assert_eq!(questions[4]["type"], serde_yaml::Value::String("file".to_string()));
    assert!(questions[4]["answer"].is_null());

    // Cleanup
    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_save_and_load_roundtrip() {
    let content = fs::read_to_string("fixtures/sample_quiz.md").expect("Cannot read fixture");
    let quiz =
        termquiz::parser::parse_quiz(&content, "sample_quiz.md", "sha256:abc123").unwrap();

    let tmp_dir = std::env::temp_dir().join("termquiz_test_roundtrip");
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap();

    // Set up state with answers
    let mut state = termquiz::state::AppState::new(quiz.clone(), tmp_dir.clone());
    state.started_at = Some("2025-01-02T10:00:00-05:00".to_string());
    state.current_question = 2;
    state.ack_data = Some(termquiz::model::AckData {
        name: "Jane".to_string(),
        agreed_at: "2025-01-02T10:00:05-05:00".to_string(),
        text_hash: "sha256:abc".to_string(),
    });
    state.answers.insert(1, termquiz::model::Answer {
        answer_type: "single".to_string(),
        selected: Some(vec!["b".to_string()]),
        text: None,
        files: None,
    });
    state.answers.insert(3, termquiz::model::Answer {
        answer_type: "short".to_string(),
        selected: None,
        text: Some("-i".to_string()),
        files: None,
    });
    state.done_marks.insert(1, true);
    state.flags.insert(3, true);
    state.hints_revealed.insert(4, 1);

    // Save
    termquiz::persist::save_state(&state).unwrap();
    assert!(tmp_dir.join("response/answers.yaml").exists());

    // Load into fresh state
    let mut state2 = termquiz::state::AppState::new(quiz, tmp_dir.clone());
    let loaded = termquiz::persist::load_state(&mut state2).unwrap();
    assert!(loaded);

    // Verify restored state
    assert_eq!(state2.current_question, 2);
    assert_eq!(state2.started_at, Some("2025-01-02T10:00:00-05:00".to_string()));
    assert!(state2.ack_data.is_some());
    assert_eq!(state2.ack_data.as_ref().unwrap().name, "Jane");

    // Q1 answer
    let a1 = state2.answers.get(&1).expect("Q1 answer missing");
    assert_eq!(a1.selected.as_ref().unwrap(), &vec!["b".to_string()]);

    // Q3 answer
    let a3 = state2.answers.get(&3).expect("Q3 answer missing");
    assert_eq!(a3.text.as_ref().unwrap(), "-i");

    // Done/flag/hint
    assert!(state2.done_marks.get(&1).copied().unwrap_or(false));
    assert!(state2.flags.get(&3).copied().unwrap_or(false));
    assert_eq!(state2.hints_revealed.get(&4).copied().unwrap_or(0), 1);

    // Cleanup
    let _ = fs::remove_dir_all(&tmp_dir);
}
