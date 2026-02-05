use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "termquiz", version, about = "Terminal-based quiz application")]
pub struct Cli {
    /// Path to repo/file, or git URL [default: .]
    #[arg(default_value = ".")]
    pub path_or_url: String,

    /// Clear saved state and start fresh
    #[arg(long)]
    pub clear: bool,

    /// Show current progress without entering TUI
    #[arg(long)]
    pub status: bool,

    /// Export current answers to file (for backup)
    #[arg(long, value_name = "path")]
    pub export: Option<String>,

    /// Directory for auto-clone [default: ~/termquiz-exams/<repo-name>]
    #[arg(long, value_name = "dir")]
    pub clone_to: Option<String>,
}
