use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frontmatter {
    pub title: Option<String>,
    pub start: DateTime<FixedOffset>,
    pub end: DateTime<FixedOffset>,
    #[serde(default)]
    pub acknowledgment: Option<AckConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckConfig {
    pub required: bool,
    pub text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Quiz {
    pub frontmatter: Frontmatter,
    pub title: String,
    pub preamble: Vec<String>,
    pub questions: Vec<Question>,
    pub quiz_file: String,
    pub quiz_hash: String,
}

#[derive(Debug, Clone)]
pub struct Question {
    pub number: u32,
    pub title: String,
    pub body_lines: Vec<BodyElement>,
    pub kind: QuestionKind,
    pub hints: Vec<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum BodyElement {
    Text(String),
    Code(String),
    Bold(String),
    Italic(String),
    InlineCode(String),
    ListItem(String),
}

#[derive(Debug, Clone)]
pub enum QuestionKind {
    SingleChoice(Vec<Choice>),
    MultiChoice(Vec<Choice>),
    Short,
    Long,
    File(FileConstraints),
}

#[derive(Debug, Clone)]
pub struct Choice {
    pub label: char,
    pub text: String,
    #[allow(dead_code)]
    pub marked: bool,
}

#[derive(Debug, Clone)]
pub struct FileConstraints {
    pub max_files: Option<u32>,
    pub max_size: Option<u64>,
    pub accept: Vec<String>,
}

impl Default for FileConstraints {
    fn default() -> Self {
        Self {
            max_files: None,
            max_size: None,
            accept: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Answer {
    #[serde(rename = "type")]
    pub answer_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckData {
    pub name: String,
    pub agreed_at: String,
    pub text_hash: String,
}
