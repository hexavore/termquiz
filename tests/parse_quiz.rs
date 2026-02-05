use std::fs;

#[test]
fn test_parse_sample_quiz() {
    let content = fs::read_to_string("fixtures/sample_quiz.md").expect("Cannot read fixture");
    let quiz = termquiz::parser::parse_quiz(&content, "sample_quiz.md", "sha256:test").unwrap();

    assert_eq!(quiz.title, "Midterm Exam: Systems Programming");
    assert_eq!(quiz.questions.len(), 5);

    // Question 1: Single choice
    let q1 = &quiz.questions[0];
    assert_eq!(q1.number, 1);
    assert!(q1.title.contains("Multiple Choice (Single)"));
    match &q1.kind {
        termquiz::model::QuestionKind::SingleChoice(choices) => {
            assert_eq!(choices.len(), 4);
            assert_eq!(choices[0].text, "exec");
            assert_eq!(choices[1].text, "fork");
            assert_eq!(choices[1].marked, true);
        }
        _ => panic!("Expected SingleChoice"),
    }

    // Question 2: Multi choice
    let q2 = &quiz.questions[1];
    assert_eq!(q2.number, 2);
    match &q2.kind {
        termquiz::model::QuestionKind::MultiChoice(choices) => {
            assert_eq!(choices.len(), 5);
        }
        _ => panic!("Expected MultiChoice"),
    }

    // Question 3: Short answer
    let q3 = &quiz.questions[2];
    assert_eq!(q3.number, 3);
    match &q3.kind {
        termquiz::model::QuestionKind::Short => {}
        _ => panic!("Expected Short"),
    }

    // Question 4: Long answer with hints
    let q4 = &quiz.questions[3];
    assert_eq!(q4.number, 4);
    match &q4.kind {
        termquiz::model::QuestionKind::Long => {}
        _ => panic!("Expected Long, got {:?}", q4.kind),
    }
    assert_eq!(q4.hints.len(), 2);

    // Question 5: File with constraints
    let q5 = &quiz.questions[4];
    assert_eq!(q5.number, 5);
    match &q5.kind {
        termquiz::model::QuestionKind::File(constraints) => {
            assert_eq!(constraints.max_files, Some(3));
            assert_eq!(constraints.max_size, Some(5 * 1024 * 1024));
            assert!(constraints.accept.contains(&".rs".to_string()));
        }
        _ => panic!("Expected File"),
    }
    assert_eq!(q5.hints.len(), 1);
}

#[test]
fn test_frontmatter_parsing() {
    let content = fs::read_to_string("fixtures/sample_quiz.md").expect("Cannot read fixture");
    let quiz = termquiz::parser::parse_quiz(&content, "test.md", "sha256:test").unwrap();

    assert!(quiz.frontmatter.acknowledgment.is_some());
    let ack = quiz.frontmatter.acknowledgment.as_ref().unwrap();
    assert!(ack.required);
    assert!(ack.text.is_some());
}

#[test]
fn test_preamble_parsing() {
    let content = fs::read_to_string("fixtures/sample_quiz.md").expect("Cannot read fixture");
    let quiz = termquiz::parser::parse_quiz(&content, "test.md", "sha256:test").unwrap();

    assert!(!quiz.preamble.is_empty());
    assert!(quiz.preamble[0].contains("Read all questions carefully"));
}
