use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "todo")]
#[command(version, about = "Context-aware task manager with TUI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new task (headless)
    #[command(alias = "-c")]
    Create(CreateArgs),

    /// List tasks with optional filters
    List(ListArgs),

    /// Show a task's full details
    Show(ShowArgs),

    /// Mark a task as done
    Complete(CompleteArgs),

    /// Delete a task
    Delete(DeleteArgs),

    /// Search tasks by title and body content
    Search(SearchArgs),
}

#[derive(Parser)]
pub struct CreateArgs {
    /// Task title (if omitted, first line of body is used, or "Untitled")
    #[arg(long)]
    pub title: Option<String>,

    /// Comma-separated tags
    #[arg(long)]
    pub tags: Option<String>,

    /// Due date/time, e.g. "2025-03-15 17:00" or "2025-03-15T17:00:00+05:00"
    #[arg(long)]
    pub due: Option<String>,

    /// Priority: critical, high, medium, low
    #[arg(long, value_parser = ["critical", "high", "medium", "low"])]
    pub priority: Option<String>,

    /// Context file or folder path (defaults to CWD)
    #[arg(long)]
    pub context_path: Option<String>,

    /// Line number (or line:col) in the context file
    #[arg(long)]
    pub context_path_line: Option<String>,

    /// Regex to locate a section in the context file
    #[arg(long)]
    pub context_path_lookfor: Option<String>,

    /// Everything after -- becomes the task body
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub body: Vec<String>,
}

#[derive(Parser)]
pub struct ListArgs {
    /// Filter by status: todo, in_progress, done, blocked, cancelled
    #[arg(long)]
    pub status: Option<String>,

    /// Filter by tag
    #[arg(long)]
    pub tag: Option<String>,

    /// Filter by priority
    #[arg(long)]
    pub priority: Option<String>,

    /// Sort by: created, due, priority, modified (default: created)
    #[arg(long, default_value = "created")]
    pub sort: String,

    /// Reverse sort order
    #[arg(long)]
    pub reverse: bool,

    /// Limit number of results
    #[arg(long)]
    pub limit: Option<usize>,
}

#[derive(Parser)]
pub struct ShowArgs {
    /// Task ID (ULID) or unique prefix
    pub id: String,
}

#[derive(Parser)]
pub struct CompleteArgs {
    /// Task ID (ULID) or unique prefix
    pub id: String,
}

#[derive(Parser)]
pub struct DeleteArgs {
    /// Task ID (ULID) or unique prefix
    pub id: String,

    /// Skip confirmation
    #[arg(long, short)]
    pub force: bool,
}

#[derive(Parser)]
pub struct SearchArgs {
    /// Search query (matched against title and body)
    pub query: String,
}
