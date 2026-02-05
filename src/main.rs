mod cli;
mod editor;
mod git;
mod model;
mod parser;
mod persist;
mod source;
mod state;
mod submit;
mod timer;
mod tui;
mod ui;

use clap::Parser;

use crate::cli::Cli;
use crate::persist::{compute_file_hash, state_dir_for};
use crate::state::{AppState, Screen};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();

    // Resolve source
    let (repo_dir, quiz_path) = source::resolve_source(
        &cli.path_or_url,
        cli.clone_to.as_deref(),
    )?;

    // Compute quiz file hash
    let quiz_hash = compute_file_hash(&quiz_path)?;

    // Read and parse quiz
    let content = std::fs::read_to_string(&quiz_path)
        .map_err(|e| format!("Cannot read quiz file: {}", e))?;

    let quiz_filename = quiz_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let quiz = parser::parse_quiz(&content, &quiz_filename, &quiz_hash)?;

    // Compute state directory
    let canonical = quiz_path
        .canonicalize()
        .unwrap_or_else(|_| quiz_path.clone());
    let state_dir = state_dir_for(&canonical);

    // Handle --clear
    if cli.clear {
        persist::clear_state(&state_dir)?;
        eprintln!("State cleared.");
    }

    // Create state
    let mut state = AppState::new(quiz, repo_dir.clone());

    // Load persisted state
    if !cli.clear {
        match persist::load_state(&mut state, &state_dir) {
            Ok(true) => {
                // State loaded successfully
            }
            Ok(false) => {
                // No saved state
            }
            Err(e) => {
                eprintln!("Warning: {}", e);
            }
        }
    }

    // Handle --status
    if cli.status {
        persist::print_status(&state);
        return Ok(());
    }

    // Handle --export
    if let Some(ref export_path) = cli.export {
        persist::export_answers(&state, export_path)?;
        eprintln!("Answers exported to {}", export_path);
        return Ok(());
    }

    // Check for existing submission
    if git::is_git_repo(&repo_dir) && git::has_existing_submission(&repo_dir) {
        state.screen = Screen::AlreadySubmitted;
    } else {
        // Determine initial screen based on time window
        let now = chrono::Utc::now();
        let start = state.quiz.frontmatter.start;
        let end = state.quiz.frontmatter.end;

        if now < start {
            state.screen = Screen::Waiting;
        } else if now > end {
            state.screen = Screen::Closed;
        } else {
            // Within time window - show preamble
            state.screen = Screen::Preamble;

            // If we already have ack data or don't need ack, and already started
            if state.started_at.is_some() {
                let needs_ack = state
                    .quiz
                    .frontmatter
                    .acknowledgment
                    .as_ref()
                    .map(|a| a.required)
                    .unwrap_or(false);

                if !needs_ack || state.ack_data.is_some() {
                    state.screen = Screen::Working;
                }
            }
        }
    }

    // Start timer
    let timer_rx = timer::spawn_timer(state.quiz.frontmatter.end);

    // Run TUI
    tui::run_tui(state, timer_rx, state_dir)?;

    Ok(())
}
