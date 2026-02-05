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

    termquiz::submit::build_response(&state, &tmp_dir).unwrap();

    // Verify response directory
    assert!(tmp_dir.join("response/meta.toml").exists());
    assert!(tmp_dir.join("response/answers.toml").exists());

    let meta = fs::read_to_string(tmp_dir.join("response/meta.toml")).unwrap();
    assert!(meta.contains("sample_quiz.md"));
    assert!(meta.contains("sha256:abc123"));

    let answers = fs::read_to_string(tmp_dir.join("response/answers.toml")).unwrap();
    assert!(answers.contains("[q1]"));
    assert!(answers.contains("[q3]"));

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
