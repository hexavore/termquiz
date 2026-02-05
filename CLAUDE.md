# termquiz - Current State

A Rust TUI application for taking timed assessments. Students clone their personal quiz repo, run `termquiz`, answer questions, and submit via git push.

## Project Status: Functional

All 11 phases from the implementation plan are complete. The application compiles with zero warnings and passes all tests.

## Architecture

| Decision | Choice |
|----------|--------|
| Platform | Linux/Ubuntu only |
| Architecture | Sync main loop + `std::thread` |
| Git ops | Shell out to `git` CLI |
| File picker | `zenity --file-selection`, TUI fallback |
| Multi/Single select | `(Multi)` keyword in H2 title |
| Markdown rendering | pulldown-cmark -> ratatui Spans |
| Submit detection | Git history + working tree |
| Frontmatter | `serde_yaml` |
| crossterm | `ratatui::crossterm` re-export |

## File Structure

```
src/
  main.rs          -- entry point, CLI orchestration
  lib.rs           -- library exports for tests
  cli.rs           -- clap derive CLI definition
  source.rs        -- resolve PATH_OR_URL, auto-clone, find quiz .md
  parser.rs        -- markdown + YAML frontmatter -> Quiz model
  model.rs         -- Quiz, Question, Choice, Answer, etc.
  state.rs         -- AppState, Screen enum, input handling
  persist.rs       -- ~/.local/state/termquiz/<hash>/ persistence
  timer.rs         -- background timer thread via mpsc
  git.rs           -- shell out to git CLI
  submit.rs        -- build response/ dir, meta.toml, answers.toml
  editor.rs        -- spawn $EDITOR, zenity file picker
  tui.rs           -- terminal init/restore, main event loop, mouse handling
  ui/
    mod.rs         -- top-level draw() dispatcher
    layout.rs      -- frame layout (30-char sidebar + main)
    titlebar.rs    -- title + countdown (yellow bold, red bg when urgent)
    sidebar.rs     -- question list: icon + number + title
    question.rs    -- question content + answer input widgets
    statusbar.rs   -- totals per status
    keybar.rs      -- key binding hints
    dialog.rs      -- confirmation dialogs, help overlay
    waiting.rs     -- waiting/closed screens
    ack.rs         -- acknowledgment screen
    result.rs      -- submitted/pushing/save-local screens
    markdown.rs    -- pulldown-cmark -> ratatui styled Lines
fixtures/
  sample_quiz.md   -- 5-question test fixture (all types)
  mc_quiz.md       -- 25-question multiple choice only
tests/
  parse_quiz.rs    -- parser tests (3 tests)
  submit_format.rs -- submission format tests (2 tests)
```

## Features Implemented

### Core
- [x] CLI with `--clear`, `--status`, `--export`, `--clone-to`
- [x] Git URL auto-clone and pull
- [x] YAML frontmatter parsing (title, start/end times, acknowledgment)
- [x] Markdown question parsing (single/multi choice, short/long answer, file upload)
- [x] `:::hint` block extraction
- [x] File constraints parsing (`max_files`, `max_size`, `accept`)

### TUI
- [x] Sidebar with status icons (·○◐✓⚑), question numbers, titles
- [x] Question rendering with markdown styling
- [x] Single choice (radio buttons)
- [x] Multi choice (checkboxes)
- [x] Short answer (inline text input)
- [x] Long answer ($EDITOR integration)
- [x] File attachment (zenity picker)
- [x] Countdown timer (yellow bold, red background when ≤2min)
- [x] Status bar with counts
- [x] Context-sensitive key bar

### Mouse Support
- [x] Click sidebar to navigate questions
- [x] Click choices to select/toggle
- [x] Scroll wheel in sidebar (navigate questions)
- [x] Scroll wheel in main area (scroll content)

### State & Persistence
- [x] Auto-save on every change
- [x] Session restore on restart
- [x] `--clear` to reset state
- [x] `--export` to backup answers

### Time Windows
- [x] Waiting screen before start time
- [x] Closed screen after end time
- [x] 2-minute warning dialog
- [x] Auto-submit on time expiry

### Submission
- [x] Build `response/` directory
- [x] `meta.toml` with timestamps, acknowledgment, hints used
- [x] `answers.toml` with all responses
- [x] File copying to `response/files/qN/`
- [x] Git add/commit/push
- [x] Push retry with exponential backoff (2s→30s, 10min timeout)
- [x] Conflict detection (already submitted)
- [x] Local save fallback with manual instructions

### Dialogs
- [x] Confirm submit (shows empty/flagged counts)
- [x] Confirm quit
- [x] Confirm hint reveal
- [x] Confirm file delete
- [x] 2-minute warning
- [x] Help overlay (full key reference)

## Key Bindings

| Key | Action |
|-----|--------|
| ↑/↓ | Navigate choices / scroll |
| ←/→ | Previous/next question |
| PgUp/PgDn | Jump 5 questions |
| Home/End | First/last question |
| Enter | Confirm selection |
| Space | Toggle multi-choice |
| Ctrl+H | Reveal hint |
| Ctrl+F | Toggle flag |
| Ctrl+E | Open editor (long answer) |
| Ctrl+A | Attach file |
| Ctrl+D | Delete attachment |
| Ctrl+S | Submit |
| Ctrl+Q | Quit |
| ? | Help |
| Esc | Close dialog |
| Mouse | Click/scroll supported |

## Running

```bash
# With sample quiz
cargo run -- ./fixtures/mc_quiz.md

# Check status without TUI
cargo run -- --status ./fixtures/mc_quiz.md

# Clear saved state
cargo run -- --clear ./fixtures/mc_quiz.md

# Export answers
cargo run -- --export answers.toml ./fixtures/mc_quiz.md
```

## Tests

```bash
cargo test
```

5 tests total:
- `test_parse_sample_quiz` - full quiz parsing
- `test_frontmatter_parsing` - YAML frontmatter
- `test_preamble_parsing` - preamble extraction
- `test_build_response` - submission directory creation
- `test_commit_message` - commit message format

## Dependencies

```toml
ratatui = "0.29"
pulldown-cmark = "0.12"
toml = "0.8"
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
sha2 = "0.10"
chrono = { version = "0.4", features = ["serde"] }
directories = "6"
clap = { version = "4", features = ["derive"] }
```

No tokio, no git2, no separate crossterm (uses ratatui re-export).

## Known Limitations

- Mouse click detection for choices is approximate (based on estimated line positions)
- No TUI fallback for file path input when zenity unavailable
- Single `.md` file per directory required (or specify path explicitly)
