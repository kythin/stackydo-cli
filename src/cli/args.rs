use clap::{ArgAction, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "stackydo")]
#[command(version, about = "Context-aware task manager with TUI — stacks to do!")]
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

    /// Update a task's fields
    Update(UpdateArgs),

    /// Mark a task as done
    Complete(CompleteArgs),

    /// Delete a task
    Delete(DeleteArgs),

    /// Search tasks by title and body content
    Search(SearchArgs),

    /// Show the context that would be captured for a new task (debugging)
    Context(ContextArgs),

    /// Show aggregate statistics across all tasks
    Stats(StatsArgs),

    /// List all known stacks with per-stack status counts
    Stacks(StacksArgs),

    /// Initialize a new stackydo workspace
    Init(InitArgs),

    /// Import tasks from stdin (JSON or YAML)
    Import(ImportArgs),
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

    /// Stack to put this task on (e.g. "work", "personal", "sprint-12")
    #[arg(long)]
    pub stack: Option<String>,

    /// Task ID that blocks this task (repeatable)
    #[arg(long, action = ArgAction::Append)]
    pub blocked_by: Vec<String>,

    /// Task ID that this task blocks (repeatable)
    #[arg(long, action = ArgAction::Append)]
    pub blocks: Vec<String>,

    /// Related task ID (repeatable)
    #[arg(long, action = ArgAction::Append)]
    pub related_to: Vec<String>,

    /// Parent task ID (makes this a subtask)
    #[arg(long)]
    pub parent: Option<String>,

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
pub struct UpdateArgs {
    /// Task ID (ULID) or unique prefix
    pub id: String,

    /// New title
    #[arg(long)]
    pub title: Option<String>,

    /// New status: todo, in_progress, done, blocked, cancelled
    #[arg(long)]
    pub status: Option<String>,

    /// New priority: critical, high, medium, low, none (clears)
    #[arg(long)]
    pub priority: Option<String>,

    /// Replace tags (comma-separated; empty string clears)
    #[arg(long)]
    pub tags: Option<String>,

    /// Set stack (empty string clears)
    #[arg(long)]
    pub stack: Option<String>,

    /// Set due date (empty string clears)
    #[arg(long)]
    pub due: Option<String>,

    /// Append a timestamped note to the body
    #[arg(long)]
    pub note: Option<String>,

    /// Task ID that blocks this task (repeatable, appends)
    #[arg(long, action = ArgAction::Append)]
    pub blocked_by: Vec<String>,

    /// Task ID that this task blocks (repeatable, appends)
    #[arg(long, action = ArgAction::Append)]
    pub blocks: Vec<String>,

    /// Related task ID (repeatable, appends)
    #[arg(long, action = ArgAction::Append)]
    pub related_to: Vec<String>,

    /// Clear all dependencies before adding new ones
    #[arg(long)]
    pub clear_deps: bool,

    /// Set parent task ID (makes this a subtask)
    #[arg(long)]
    pub parent: Option<String>,

    /// Text to append to body (everything after --)
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

    /// Filter by stack
    #[arg(long)]
    pub stack: Option<String>,

    /// Sort by: created, due, priority, modified (default: created)
    #[arg(long, default_value = "created")]
    pub sort: String,

    /// Reverse sort order
    #[arg(long)]
    pub reverse: bool,

    /// Limit number of results
    #[arg(long)]
    pub limit: Option<usize>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Show only overdue tasks (due before now, not done/cancelled)
    #[arg(long)]
    pub overdue: bool,

    /// Filter tasks due before this date
    #[arg(long)]
    pub due_before: Option<String>,

    /// Filter tasks due after this date
    #[arg(long)]
    pub due_after: Option<String>,

    /// Filter tasks due this week (Monday–Sunday)
    #[arg(long)]
    pub due_this_week: bool,

    /// Group output by field (e.g. "stack")
    #[arg(long)]
    pub group_by: Option<String>,
}

#[derive(Parser)]
pub struct ShowArgs {
    /// Task ID (ULID) or unique prefix
    pub id: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Parser)]
pub struct CompleteArgs {
    /// Task ID (ULID) or unique prefix (omit for bulk mode)
    pub id: Option<String>,

    /// Bulk: filter by status
    #[arg(long)]
    pub status: Option<String>,

    /// Bulk: filter by tag
    #[arg(long)]
    pub tag: Option<String>,

    /// Bulk: filter by stack
    #[arg(long)]
    pub stack: Option<String>,

    /// Required safety flag for bulk operations
    #[arg(long)]
    pub all: bool,
}

#[derive(Parser)]
pub struct DeleteArgs {
    /// Task ID (ULID) or unique prefix (omit for bulk mode)
    pub id: Option<String>,

    /// Skip confirmation
    #[arg(long, short)]
    pub force: bool,

    /// Bulk: filter by status
    #[arg(long)]
    pub status: Option<String>,

    /// Bulk: filter by tag
    #[arg(long)]
    pub tag: Option<String>,

    /// Bulk: filter by stack
    #[arg(long)]
    pub stack: Option<String>,

    /// Required safety flag for bulk operations
    #[arg(long)]
    pub all: bool,
}

#[derive(Parser)]
pub struct SearchArgs {
    /// Search query (matched against title and body)
    pub query: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Parser)]
pub struct ContextArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Parser)]
pub struct StatsArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Parser)]
pub struct StacksArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Parser)]
pub struct InitArgs {
    /// Override storage directory path
    #[arg(long)]
    pub dir: Option<String>,

    /// Write a .stackydo-context in CWD pointing to --dir (implies --dir)
    #[arg(long)]
    pub here: bool,

    /// Non-interactive mode, accept defaults
    #[arg(long, short)]
    pub yes: bool,

    /// Initialize git in the storage directory
    #[arg(long)]
    pub git: bool,
}

#[derive(Parser)]
pub struct ImportArgs {
    /// Input format: json or yaml (default: json)
    #[arg(long, default_value = "json")]
    pub format: String,
}
