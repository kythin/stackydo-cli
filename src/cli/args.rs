use clap::{value_parser, ArgAction, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "stackydo")]
#[command(version, about = "Context-aware task manager — stacks to do!")]
#[command(arg_required_else_help = true)]
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

    /// Register stackydo-mcp with Claude Code via `claude mcp add`
    #[command(name = "mcp-setup")]
    McpSetup(McpSetupArgs),

    /// Discover and list all stackydo workspaces
    #[command(name = "list-workspaces", alias = "lw")]
    ListWorkspaces(ListWorkspacesArgs),

    /// Move or copy tasks between workspaces
    Migrate(MigrateArgs),
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

    /// New status: todo, in_progress, done, blocked, cancelled, on_hold, in_review
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

    /// Replace entire body content (empty string clears it)
    #[arg(long)]
    pub body_replace: Option<String>,

    /// Sed-style substitution on body: s/pattern/replacement/[g]
    #[arg(long)]
    pub body_sub: Option<String>,

    /// Append a timestamped note to the body
    #[arg(long)]
    pub note: Option<String>,

    /// Preview resulting body without saving
    #[arg(long)]
    pub dry_run: bool,

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
    /// Filter by status: todo, in_progress, done, blocked, cancelled, on_hold, in_review
    #[arg(long)]
    pub status: Option<String>,

    /// Filter by stage: backlog, active, archive
    #[arg(long)]
    pub stage: Option<String>,

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

    /// Max results to return (default: 50, 0 = no limit)
    #[arg(long)]
    pub limit: Option<usize>,

    /// Skip the first N results (0-indexed, default: 0). E.g. --offset 50 --limit 50 shows results 51-100
    #[arg(long, default_value = "0")]
    pub offset: usize,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Include full task body in JSON output (omitted by default)
    #[arg(long)]
    pub full: bool,

    /// Show only overdue tasks (due before now, not in archive stage)
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

    /// Filter by status: todo, in_progress, done, blocked, cancelled, on_hold, in_review
    #[arg(long)]
    pub status: Option<String>,

    /// Filter by stage: backlog, active, archive
    #[arg(long)]
    pub stage: Option<String>,

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

    /// Max results to return (default: 50, 0 = no limit)
    #[arg(long)]
    pub limit: Option<usize>,

    /// Skip the first N results (0-indexed, default: 0). E.g. --offset 50 --limit 50 shows results 51-100
    #[arg(long, default_value = "0")]
    pub offset: usize,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Include full task body in JSON output (omitted by default)
    #[arg(long)]
    pub full: bool,

    /// Show only overdue tasks (due before now, not in archive stage)
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

    /// Write a stackydo.json in CWD pointing to --dir (implies --dir)
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

#[derive(Parser)]
pub struct McpSetupArgs {
    /// MCP scope: project, user, or local (default: project)
    #[arg(long, default_value = "project")]
    pub scope: Option<String>,

    /// Name to register the server under (default: stackydo)
    #[arg(long, default_value = "stackydo")]
    pub name: Option<String>,
}

#[derive(Parser)]
pub struct ListWorkspacesArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Parser)]
pub struct MigrateArgs {
    /// Source workspace path (directory, stackydo.json, or store dir)
    #[arg(long)]
    pub source: Option<String>,

    /// Destination workspace path
    #[arg(long)]
    pub dest: Option<String>,

    /// Filter by stack name (repeatable)
    #[arg(long, action = ArgAction::Append)]
    pub stack: Vec<String>,

    /// Select all tasks from matched stacks
    #[arg(long)]
    pub all: bool,

    /// Specific task ID or prefix (repeatable)
    #[arg(long, action = ArgAction::Append)]
    pub task: Vec<String>,

    /// Move tasks (delete from source after copying)
    #[arg(long = "move", conflicts_with = "copy")]
    pub r#move: bool,

    /// Copy tasks (keep in both workspaces)
    #[arg(long, conflicts_with = "move")]
    pub copy: bool,

    /// Preview only — don't make any changes
    #[arg(long)]
    pub dry_run: bool,

    /// Skip confirmation; overwrite conflicts
    #[arg(long, short)]
    pub force: bool,

    /// Create git commits for rollback (auto-detected if omitted)
    #[arg(long, value_parser = value_parser!(bool), num_args = 0..=1, default_missing_value = "true")]
    pub git_commit: Option<bool>,
}
