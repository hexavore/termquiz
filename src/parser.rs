use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use crate::model::*;

pub fn parse_quiz(content: &str, quiz_file: &str, quiz_hash: &str) -> Result<Quiz, String> {
    let (frontmatter, body) = split_frontmatter(content)?;
    let fm: Frontmatter =
        serde_yaml::from_str(&frontmatter).map_err(|e| format!("Invalid frontmatter: {}", e))?;

    let (title, preamble, questions) = parse_body(&body)?;

    let title = fm.title.clone().unwrap_or(title);

    Ok(Quiz {
        frontmatter: fm,
        title,
        preamble,
        questions,
        quiz_file: quiz_file.to_string(),
        quiz_hash: quiz_hash.to_string(),
    })
}

fn split_frontmatter(content: &str) -> Result<(String, String), String> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return Err("Quiz file must start with YAML frontmatter (---)".to_string());
    }

    let after_first = &trimmed[3..];
    let end_pos = after_first
        .find("\n---")
        .ok_or_else(|| "No closing --- for frontmatter".to_string())?;

    let fm = after_first[..end_pos].trim().to_string();
    let body = after_first[end_pos + 4..].to_string();

    Ok((fm, body))
}

fn parse_body(body: &str) -> Result<(String, Vec<String>, Vec<Question>), String> {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(body, opts);
    let events: Vec<Event> = parser.collect();

    let mut title = String::new();
    let mut preamble: Vec<String> = Vec::new();
    let mut questions: Vec<Question> = Vec::new();

    let mut in_h1 = false;
    let mut in_h2 = false;
    let mut current_h2_text = String::new();
    let mut seen_h2 = false;

    // Collect content between questions as raw sections
    let mut current_choices: Vec<Choice> = Vec::new();
    let mut current_kind: Option<QuestionKind> = None;
    let mut current_hints: Vec<String> = Vec::new();
    let mut current_body: Vec<BodyElement> = Vec::new();
    let mut in_blockquote = false;
    let mut blockquote_text = String::new();
    let mut in_hint_block = false;
    let mut hint_text = String::new();
    let mut choice_index: u8 = 0;
    let mut in_list_item = false;
    let mut list_item_text = String::new();
    let mut task_list_checked: Option<bool> = None;
    let mut in_paragraph = false;
    let mut paragraph_text = String::new();
    let mut in_code_block = false;
    let mut code_block_text = String::new();

    let mut i = 0;
    while i < events.len() {
        let event = &events[i];
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                match level {
                    pulldown_cmark::HeadingLevel::H1 => {
                        in_h1 = true;
                    }
                    pulldown_cmark::HeadingLevel::H2 => {
                        // Finish previous question if any
                        if seen_h2 {
                            finalize_question(
                                &current_h2_text,
                                &mut questions,
                                &mut current_choices,
                                &mut current_kind,
                                &mut current_hints,
                                &mut current_body,
                                &mut choice_index,
                            )?;
                        }
                        in_h2 = true;
                        current_h2_text = String::new();
                        seen_h2 = true;
                    }
                    _ => {}
                }
            }
            Event::End(TagEnd::Heading(level)) => {
                match level {
                    pulldown_cmark::HeadingLevel::H1 => {
                        in_h1 = false;
                    }
                    pulldown_cmark::HeadingLevel::H2 => {
                        in_h2 = false;
                    }
                    _ => {}
                }
            }
            Event::Start(Tag::BlockQuote(_)) => {
                in_blockquote = true;
                blockquote_text = String::new();
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                in_blockquote = false;
                let trimmed = blockquote_text.trim().to_string();
                if seen_h2 {
                    if trimmed == "short" {
                        current_kind = Some(QuestionKind::Short);
                    } else if trimmed == "long" {
                        current_kind = Some(QuestionKind::Long);
                    } else if trimmed.starts_with("file") {
                        current_kind = Some(QuestionKind::File(parse_file_constraints(&trimmed)));
                    }
                }
            }
            Event::Start(Tag::List(_)) => {}
            Event::End(TagEnd::List(_)) => {}
            Event::Start(Tag::Item) => {
                in_list_item = true;
                list_item_text = String::new();
                task_list_checked = None;
            }
            Event::End(TagEnd::Item) => {
                in_list_item = false;
                if seen_h2 {
                    if let Some(checked) = task_list_checked {
                        let label = (b'a' + choice_index) as char;
                        current_choices.push(Choice {
                            label,
                            text: list_item_text.trim().to_string(),
                            marked: checked,
                        });
                        choice_index += 1;
                    } else if !list_item_text.trim().is_empty() {
                        current_body.push(BodyElement::ListItem(
                            list_item_text.trim().to_string(),
                        ));
                    }
                }
                task_list_checked = None;
            }
            Event::TaskListMarker(checked) => {
                task_list_checked = Some(*checked);
            }
            Event::Start(Tag::Paragraph) => {
                in_paragraph = true;
                paragraph_text = String::new();
            }
            Event::End(TagEnd::Paragraph) => {
                in_paragraph = false;
                let text = paragraph_text.trim().to_string();

                if in_hint_block {
                    if !text.is_empty() {
                        if !hint_text.is_empty() {
                            hint_text.push('\n');
                        }
                        hint_text.push_str(&text);
                    }
                } else if in_blockquote {
                    // blockquote_text is already set in the Text handler;
                    // only overwrite if paragraph_text collected something
                    if !text.is_empty() {
                        blockquote_text = text;
                    }
                } else if !text.is_empty() {
                    // Check for hint markers
                    if text.starts_with(":::hint") {
                        in_hint_block = true;
                        hint_text = String::new();
                    } else if text == ":::" && in_hint_block {
                        // end hint - handled below
                    } else if !seen_h2 && !in_h1 {
                        preamble.push(text);
                    } else if seen_h2 {
                        current_body.push(BodyElement::Text(text));
                    }
                }
            }
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
                code_block_text = String::new();
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                if seen_h2 {
                    current_body.push(BodyElement::Code(code_block_text.clone()));
                }
            }
            Event::Text(text) => {
                let t = text.to_string();

                if in_h1 {
                    title = t;
                } else if in_h2 {
                    current_h2_text.push_str(&t);
                } else if in_code_block {
                    code_block_text.push_str(&t);
                } else if in_blockquote {
                    blockquote_text.push_str(&t);
                } else if in_list_item {
                    list_item_text.push_str(&t);
                } else if in_paragraph {
                    // Check for :::hint and ::: markers
                    if t.trim().starts_with(":::hint") {
                        in_hint_block = true;
                        hint_text = String::new();
                        paragraph_text = String::new();
                    } else if t.trim() == ":::" && in_hint_block {
                        in_hint_block = false;
                        if !hint_text.is_empty() {
                            if seen_h2 {
                                current_hints.push(hint_text.trim().to_string());
                            }
                        }
                        hint_text = String::new();
                        paragraph_text = String::new();
                    } else if in_hint_block {
                        // Accumulate hint text
                        if !hint_text.is_empty() {
                            hint_text.push(' ');
                        }
                        hint_text.push_str(&t);
                    } else {
                        paragraph_text.push_str(&t);
                    }
                }
            }
            Event::Code(code) => {
                let c = format!("`{}`", code);
                if in_paragraph {
                    paragraph_text.push_str(&c);
                } else if in_list_item {
                    list_item_text.push_str(&c);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if in_paragraph {
                    paragraph_text.push(' ');
                }
            }
            Event::Rule => {
                // Horizontal rule - ignore (visual separator)
            }
            _ => {}
        }
        i += 1;
    }

    // Finalize last question
    if seen_h2 {
        finalize_question(
            &current_h2_text,
            &mut questions,
            &mut current_choices,
            &mut current_kind,
            &mut current_hints,
            &mut current_body,
            &mut choice_index,
        )?;
    }

    Ok((title, preamble, questions))
}

fn finalize_question(
    h2_text: &str,
    questions: &mut Vec<Question>,
    choices: &mut Vec<Choice>,
    kind: &mut Option<QuestionKind>,
    hints: &mut Vec<String>,
    body: &mut Vec<BodyElement>,
    choice_index: &mut u8,
) -> Result<(), String> {
    let (number, title) = parse_h2_title(h2_text)?;

    let is_multi = title.contains("(Multi)");

    let final_kind = if !choices.is_empty() {
        if is_multi {
            QuestionKind::MultiChoice(std::mem::take(choices))
        } else {
            QuestionKind::SingleChoice(std::mem::take(choices))
        }
    } else {
        kind.take().unwrap_or(QuestionKind::Short)
    };

    questions.push(Question {
        number,
        title: title.to_string(),
        body_lines: std::mem::take(body),
        kind: final_kind,
        hints: std::mem::take(hints),
    });

    *choice_index = 0;
    Ok(())
}

fn parse_h2_title(text: &str) -> Result<(u32, String), String> {
    let trimmed = text.trim();
    // Expected format: "1. Title text"
    if let Some(dot_pos) = trimmed.find('.') {
        let num_str = trimmed[..dot_pos].trim();
        let title = trimmed[dot_pos + 1..].trim().to_string();
        let number: u32 = num_str
            .parse()
            .map_err(|_| format!("Invalid question number in heading: {}", trimmed))?;
        Ok((number, title))
    } else {
        Err(format!(
            "Question heading must be in format '## N. Title', got: {}",
            trimmed
        ))
    }
}

fn parse_file_constraints(text: &str) -> FileConstraints {
    let mut constraints = FileConstraints::default();

    // Parse "file(max_files: 3, max_size: 5MB, accept: .rs)"
    if let Some(start) = text.find('(') {
        if let Some(end) = text.rfind(')') {
            let params = &text[start + 1..end];
            for param in params.split(',') {
                let param = param.trim();
                if let Some((key, value)) = param.split_once(':') {
                    let key = key.trim();
                    let value = value.trim();
                    match key {
                        "max_files" => {
                            constraints.max_files = value.parse().ok();
                        }
                        "max_size" => {
                            constraints.max_size = parse_size(value);
                        }
                        "accept" => {
                            constraints.accept =
                                value.split_whitespace().map(|s| s.to_string()).collect();
                            if constraints.accept.is_empty() {
                                constraints.accept.push(value.to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    constraints
}

fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim().to_uppercase();
    if let Some(num) = s.strip_suffix("GB") {
        num.trim().parse::<u64>().ok().map(|n| n * 1024 * 1024 * 1024)
    } else if let Some(num) = s.strip_suffix("MB") {
        num.trim().parse::<u64>().ok().map(|n| n * 1024 * 1024)
    } else if let Some(num) = s.strip_suffix("KB") {
        num.trim().parse::<u64>().ok().map(|n| n * 1024)
    } else if let Some(num) = s.strip_suffix('B') {
        num.trim().parse::<u64>().ok()
    } else {
        s.parse::<u64>().ok()
    }
}
