# termquiz — Specification

A Rust TUI application for taking timed assessments. Students clone their personal quiz repo, run `termquiz`, answer questions, and submit via git push. No grading logic — just input collection and submission.

---

## Installation & Invocation

```bash
cargo install termquiz

# Run in a git repo containing exactly one .md quiz file
cd ~/exams/abc123def456
termquiz

# Or specify path/URL explicitly
termquiz ./path/to/repo
termquiz ./specific-quiz.md
termquiz git@github.com:org/exams.git
termquiz https://github.com/org/exams.git
```

---

## Quiz Markdown Format

### Frontmatter

```yaml
---
title: "Midterm Exam: Systems Programming"
start: 2025-01-02T10:00:00-05:00
end: 2025-01-02T12:00:00-05:00

acknowledgment:
  required: true
  text: |
    I affirm that I will complete this exam without assistance from 
    any other person or unauthorized resource. I understand that any 
    violation of academic integrity will result in disciplinary action.
---
```

| Field | Required | Description |
|-------|----------|-------------|
| `title` | No | Quiz title (falls back to H1 if omitted) |
| `start` | Yes | ISO 8601 datetime with timezone |
| `end` | Yes | ISO 8601 datetime with timezone |
| `acknowledgment.required` | No | If true, must complete acknowledgment before starting |
| `acknowledgment.text` | No | Custom honor code text (required if `required: true`) |

### Question Format

````markdown
# Midterm Exam: Systems Programming

Welcome to the midterm. Read all questions carefully before starting.

---

## 1. Multiple Choice (Single)

Which system call creates a new process?

- [ ] exec
- [x] fork
- [ ] spawn  
- [ ] clone

---

## 2. Multiple Choice (Multi)

Select all valid Rust integer types:

- [x] i32
- [x] u64
- [ ] int
- [x] usize
- [ ] long

---

## 3. Short Answer

What flag makes `grep` case-insensitive?

> short

---

## 4. Long Answer

Explain how the borrow checker prevents data races at compile time.

> long

:::hint
Consider what happens when multiple references exist.
:::

:::hint  
Think about mutable vs immutable borrows.
:::

---

## 5. File Submission

Submit your implementation of the linked list.

> file(max_files: 3, max_size: 5MB, accept: .rs)

:::hint
Remember to handle the empty list case.
:::

---
````

### Format Rules

- `# H1` — Quiz title (exactly one)
- Text before first `## H2` — Preamble (shown on start screen)
- `---` — Optional visual separator (ignored by parser)
- `## H2` — Question delimiter; format: `## <number>. <title>`
- `- [ ]` / `- [x]` — Choice (x marks correct answer for reference; client ignores correctness)
- `> short` — Short answer field (single line input)
- `> long` — Long answer field (multi-line editor)
- `> file(...)` — File upload with params: `max_files`, `max_size`, `accept`
- `:::hint` / `:::` — Collapsible hint block (multiple allowed per question)

---

## TUI Layout

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Midterm Exam: Systems Programming                       [00:42:17 remaining] │
├───────────┬─────────────────────────────────────────────────────────────────┤
│ Questions │                                                                 │
│           │  ## 3. Short Answer                                             │
│  1.  ✓    │                                                                 │
│  2.  ✓    │  What flag makes `grep` case-insensitive?                       │
│  3.  ◐    │                                                                 │
│  4.  ○    │  ┌─────────────────────────────────────────────────────────┐    │
│  5.  ○    │  │ -i                                                      │    │
│  6.  ⚑    │  └─────────────────────────────────────────────────────────┘    │
│  7.  ○    │                                                                 │
│  8.  ·    │  [Ctrl+H] Show hint (2 available)                               │
│  9.  ·    │                                                                 │
│ 10.  ·    │                                                                 │
│ 11.  ·    │                                                                 │
│ 12.  ·    │                                                                 │
│           │                                                                 │
│           │                                                                 │
├───────────┴─────────────────────────────────────────────────────────────────┤
│ ✓ 2 done   ◐ 1 partial   ⚑ 1 flagged   ○ 4 empty   · 4 unread    [?] help  │
├─────────────────────────────────────────────────────────────────────────────┤
│ ←/→ navigate   PgUp/PgDn jump 5   Ctrl+F flag   Ctrl+S submit   Ctrl+Q quit │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Status Icons & Colors

| Icon | Color | Meaning |
|------|-------|---------|
| `·` | dim gray | Unread — never viewed |
| `○` | white | Empty — viewed, no input |
| `◐` | blue | Partial — incomplete (e.g., 1/3 files) |
| `✓` | green | Done — complete response |
| `⚑` | red | Flagged — marked for review |

### Status Bar

Shows running totals for each state, always visible at bottom.

---

## Key Bindings

| Key | Action | Confirm Dialog? |
|-----|--------|-----------------|
| `↑` | Previous question in sidebar | |
| `↓` | Next question in sidebar | |
| `←` | Previous question | |
| `→` | Next question | |
| `PgUp` | Jump 5 questions back | |
| `PgDn` | Jump 5 questions forward | |
| `Home` | Jump to first question | |
| `End` | Jump to last question | |
| `Enter` | Confirm selection (multiple choice) | |
| `Space` | Toggle option (multi-select) | |
| `Tab` | Next input field | |
| `Ctrl+H` | Reveal next hint | **Yes** — "Reveal hint? This will be recorded." |
| `Ctrl+F` | Toggle flagged status | |
| `Ctrl+E` | Open $EDITOR for long answers | |
| `Ctrl+A` | Attach file (opens path prompt) | |
| `Ctrl+D` | Delete selected attachment | **Yes** — "Delete {filename}?" |
| `Ctrl+S` | Submit quiz | **Yes** — "Submit? You have X unanswered." |
| `Ctrl+Q` | Quit (state preserved) | **Yes** — "Quit? Progress is saved locally." |
| `?` | Help overlay | |
| `Esc` | Close dialog / cancel | |

---

## Screens & Dialogs

### Waiting Screen (Before Start Time)

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│                Midterm Exam: Systems Programming                │
│                                                                 │
│                    Quiz opens in 2h 15m 30s                     │
│                                                                 │
│                       [Ctrl+Q] Exit                             │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Acknowledgment Screen

When `acknowledgment.required: true`:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│                        Midterm Exam: Systems Programming                    │
│                                                                             │
│  ───────────────────────────────────────────────────────────────────────── │
│                                                                             │
│   I affirm that I will complete this exam without assistance from           │
│   any other person or unauthorized resource. I understand that any          │
│   violation of academic integrity will result in disciplinary action.       │
│                                                                             │
│  ───────────────────────────────────────────────────────────────────────── │
│                                                                             │
│   Type your full name to acknowledge:                                       │
│                                                                             │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │ Jane Smith                                                          │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│   [x] I have read and agree to the above statement                          │
│                                                                             │
│                      [ OK ]              [ Cancel ]                         │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

- **OK enabled only when:** name ≥ 2 characters AND checkbox checked
- **Name validation:** ≥2 characters, no further verification

### Confirmation Dialog (Generic)

```
┌─────────────────────────────────────┐
│                                     │
│   Submit your quiz?                 │
│                                     │
│   2 questions are still empty.      │
│   1 question is flagged.            │
│                                     │
│   [Enter] Confirm    [Esc] Cancel   │
│                                     │
└─────────────────────────────────────┘
```

### 2-Minute Warning

```
┌─────────────────────────────────────────┐
│                                         │
│   ⚠  2 MINUTES REMAINING               │
│                                         │
│   Your quiz will auto-submit when       │
│   time expires. Save your work.         │
│                                         │
│            [Enter] Continue             │
│                                         │
└─────────────────────────────────────────┘
```

### Already Submitted Screen

```
┌─────────────────────────────────────────┐
│                                         │
│   ✓ Quiz Already Submitted             │
│                                         │
│   Submitted: Jan 2, 2025 at 11:47 AM   │
│                                         │
│   You cannot modify your submission.    │
│                                         │
│            [Enter] Exit                 │
│                                         │
└─────────────────────────────────────────┘
```

### Push Failure — Retrying

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│   ⚠  Submission Failed — Retrying                              │
│                                                                 │
│   Could not reach git server.                                   │
│                                                                 │
│   Attempt 3 of ∞    Retrying in 5s...    [08:42 until timeout] │
│                                                                 │
│   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━                        │
│                                                                 │
│                     [Esc] Cancel and keep working               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Push Failure — Timeout (10 minutes)

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│   ✗  Submission Failed — Saved Locally                         │
│                                                                 │
│   Your answers have been saved to:                              │
│   ~/termquiz-exams/abc123/response/                             │
│                                                                 │
│   To submit manually, run:                                      │
│                                                                 │
│   cd ~/termquiz-exams/abc123                                    │
│   git add response/                                             │
│   git commit -m "termquiz: manual submit"                       │
│   git push                                                      │
│                                                                 │
│   Contact your instructor if you need assistance.               │
│                                                                 │
│                        [Enter] Exit                             │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Quiz Closed Screen

```
┌─────────────────────────────────────────┐
│                                         │
│   ✗  Quiz Closed                        │
│                                         │
│   The submission deadline has passed.   │
│                                         │
│            [Enter] Exit                 │
│                                         │
└─────────────────────────────────────────┘
```

---

## Time Window Behavior

| Condition | Behavior |
|-----------|----------|
| Before `start` | Show "Quiz opens in X" countdown, cannot proceed |
| During window | Normal operation, countdown shows time until `end` |
| T-2:00 | Warning dialog, status bar flashes |
| T-0:00 | Auto-submit immediately, no further input |
| After `end` | Show "Quiz closed", cannot start or submit |

**Timer:** Countdown only (no absolute times displayed). Trust local system clock.

---

## Submission Rules

- Student can quit (`Ctrl+Q`) and restart termquiz unlimited times within exam window
- Local state persists — answers are never lost between sessions
- **One push policy:** Once `Ctrl+S` → Confirm → Push succeeds, quiz is final
- Subsequent runs detect existing submission and show "Already submitted" screen
- Detection: Check for existing `response/` directory with valid `meta.toml` in git history

---

## Submission Format

On submit, termquiz creates a `response/` directory in the repo:

```
repo/
├── abc123def456.md          # Original quiz (untouched)
└── response/
    ├── meta.toml            # Submission metadata
    ├── answers.toml         # All text responses
    └── files/
        └── q5/
            ├── linked_list.rs
            └── tests.rs
```

### meta.toml

```toml
quiz_file = "abc123def456.md"
quiz_hash = "sha256:9f86d08..."   # Hash of original quiz file
started_at = "2025-01-02T10:01:23-05:00"
submitted_at = "2025-01-02T11:23:45-05:00"
termquiz_version = "0.1.0"

[acknowledgment]
name = "Jane Smith"
agreed_at = "2025-01-02T10:01:23-05:00"
text_hash = "sha256:a1b2c3..."  # Hash of the acknowledgment text

[hints_used]
q4 = 2
q5 = 1
```

### answers.toml

```toml
[q1]
type = "single"
selected = ["b"]   # 0-indexed: a, b, c, d...

[q2]
type = "multi"
selected = ["a", "b", "d"]

[q3]
type = "short"
text = "-i"

[q4]
type = "long"
text = """
The borrow checker enforces that at any given time...
(multi-line content)
"""

[q5]
type = "file"
files = ["files/q5/linked_list.rs", "files/q5/tests.rs"]
```

### Commit Message

```
termquiz: submit abc123def456

Started: 2025-01-02T10:01:23-05:00
Submitted: 2025-01-02T11:23:45-05:00
Questions: 12 (8 complete, 2 partial, 1 flagged, 1 empty)
```

---

## Local State Persistence

**Location:** `~/.local/state/termquiz/<repo-path-hash>/`

```
~/.local/state/termquiz/a1b2c3d4/
├── session.toml      # Started time, current question, acknowledgment
├── answers.toml      # Work in progress (same format as submission)
└── files/            # Staged file uploads
```

- Auto-save on every change
- Restored automatically when re-running `termquiz` in same repo
- Cleared only after successful `git push`

---

## CLI Interface

```bash
termquiz [OPTIONS] [PATH_OR_URL]

Arguments:
  [PATH_OR_URL]  Path to repo/file, or git URL [default: .]

Examples:
  termquiz                                    # Current directory
  termquiz ./path/to/repo                     # Local path
  termquiz ./specific-quiz.md                 # Specific file
  termquiz git@github.com:org/exams.git       # Auto-clone via SSH
  termquiz https://github.com/org/exams.git   # Auto-clone via HTTPS

Options:
  --clear          Clear saved state and start fresh
  --status         Show current progress without entering TUI
  --export <path>  Export current answers to file (for backup)
  --clone-to <dir> Directory for auto-clone [default: ~/termquiz-exams/<repo-name>]
  --version        Print version
  --help           Print help

Environment:
  EDITOR           Editor for long answers (default: vim)
  TERMQUIZ_STATE   Override state directory
```

### Auto-Clone Behavior

- If argument looks like a git URL → clone to `--clone-to` directory
- If directory already exists and is a git repo → `git pull` and continue
- If directory exists but isn't a repo → error
- After clone/pull, proceed normally

---

## State Machine

```
┌─────────┐
│  START  │
└────┬────┘
     ▼
┌─────────────┐  git URL    ┌─────────┐
│ RESOLVE SRC │────────────▶│  CLONE  │
└──────┬──────┘             └────┬────┘
       │ local path              │
       ▼◀────────────────────────┘
┌─────────┐    parse error    ┌─────────┐
│  PARSE  │──────────────────▶│  ERROR  │
└────┬────┘                   └─────────┘
     ▼
┌─────────────┐  before start  ┌──────────┐
│ CHECK TIME  │───────────────▶│ WAITING  │ (countdown to start)
└──────┬──────┘                └──────────┘
       │ in window      after end
       │◀──────────────────────┬──────────┐
       │                       ▼          │
       │                 ┌──────────┐     │
       │                 │  CLOSED  │─────┘
       │                 └──────────┘
       ▼
┌─────────────┐  already done  ┌───────────┐
│ CHECK SUBMIT│───────────────▶│ SUBMITTED │ (show summary, exit)
└──────┬──────┘                └───────────┘
       │ not yet submitted
       ▼
┌─────────────┐  required      ┌─────────┐   cancel   ┌──────┐
│   PREAMBLE  │───────────────▶│  ACK    │───────────▶│ EXIT │
└──────┬──────┘                └────┬────┘            └──────┘
       │ not required               │ confirmed
       ▼◀───────────────────────────┘
┌─────────┐◀─────────────────────────┐
│ WORKING │  navigate, answer, flag  │
└────┬────┴──────────────────────────┘
     │ Ctrl+S or timer=0
     ▼
┌─────────┐    cancel    ┌─────────┐
│ CONFIRM │─────────────▶│ WORKING │
└────┬────┘              └─────────┘
     │ confirm
     ▼
┌─────────┐  failure (≤10min)  ┌─────────┐
│ PUSHING │───────────────────▶│  RETRY  │
└────┬────┘                    └────┬────┘
     │ success                      │ retry
     │         ┌────────────────────┘
     │         │    failure (>10min)
     │         ▼
     │    ┌──────────┐
     │    │ SAVE     │ (local save, manual instructions)
     │    │ LOCAL    │
     │    └────┬─────┘
     │         │
     ▼         ▼
┌─────────────────┐
│      DONE       │  (clear local state, exit)
└─────────────────┘
```

---

## Error Handling

| Scenario | Behavior |
|----------|----------|
| No .md file in repo | Exit with error listing expected location |
| Multiple .md files | Exit with error; must specify which one |
| Parse error in markdown | Show error with line number, exit |
| Invalid frontmatter | Show specific field error, exit |
| Git push fails (network) | Retry for up to 10 minutes, then save locally |
| Git push fails (conflict) | Error: "Quiz already submitted from another session" |
| File too large | Reject with size limit message |
| Wrong file type | Reject with allowed extensions message |
| Too many files | Reject with max files message |
| State file corruption | Option to clear and restart |
| Quiz not yet open | Show waiting screen with countdown |
| Quiz already closed | Show closed screen, exit |
| Already submitted | Show submitted screen, exit |

---

## Dependencies (suggested)

```toml
[dependencies]
ratatui = "0.27"           # TUI framework
crossterm = "0.27"         # Terminal backend
tokio = { version = "1", features = ["full"] }
git2 = "0.19"              # Git operations
pulldown-cmark = "0.11"    # Markdown parsing
toml = "0.8"               # Config/state serialization
serde = { version = "1", features = ["derive"] }
sha2 = "0.10"              # Quiz file hashing
chrono = { version = "0.4", features = ["serde"] }
directories = "5"          # XDG paths
clap = { version = "4", features = ["derive"] }
```

---

## Implementation Notes

1. **Markdown Parser:** Use pulldown-cmark for base parsing, then custom logic to extract question structure, hints, and answer types.

2. **Git Operations:** Use git2 (libgit2) for clone, commit, push. Handle SSH and HTTPS auth via system credentials.

3. **State Persistence:** Write to state files on every change (debounced). Use atomic writes to prevent corruption.

4. **Timer:** Run a background task that ticks every second. At T-2:00, trigger warning. At T-0:00, force submit flow.

5. **File Handling:** Copy attached files to state directory immediately. On submit, copy to `response/files/`. Validate size/type on attach.

6. **Multi-select vs Single-select:** Detect based on number of `[x]` markers in original markdown. If >1 correct answer marked, it's multi-select.
