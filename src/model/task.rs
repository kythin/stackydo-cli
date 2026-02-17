use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Task lifecycle status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
    Blocked,
    Cancelled,
}

impl std::str::FromStr for TaskStatus {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "todo" => Ok(Self::Todo),
            "in_progress" | "inprogress" | "doing" => Ok(Self::InProgress),
            "done" => Ok(Self::Done),
            "blocked" => Ok(Self::Blocked),
            "cancelled" | "canceled" => Ok(Self::Cancelled),
            _ => Err(format!(
                "Invalid status: {s}. Use: todo, in_progress, done, blocked, cancelled"
            )),
        }
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Todo => write!(f, "todo"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Done => write!(f, "done"),
            Self::Blocked => write!(f, "blocked"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Priority levels (ordered high to low)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Critical => write!(f, "critical"),
            Self::High => write!(f, "high"),
            Self::Medium => write!(f, "medium"),
            Self::Low => write!(f, "low"),
        }
    }
}

impl std::str::FromStr for Priority {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "critical" => Ok(Self::Critical),
            "high" => Ok(Self::High),
            "medium" => Ok(Self::Medium),
            "low" => Ok(Self::Low),
            _ => Err(format!("Invalid priority: {s}. Use: critical, high, medium, low")),
        }
    }
}

/// Relationship type between tasks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    BlockedBy,
    Blocks,
    RelatedTo,
}

/// A dependency link to another task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub task_id: String,
    pub dep_type: DependencyType,
}

/// Context captured automatically at task creation time
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextInfo {
    /// The file or directory path this task relates to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// Line number in the context file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,

    /// Column position in the context file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,

    /// Regex pattern to locate relevant section in context file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lookfor: Option<String>,

    /// Git branch at creation time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,

    /// Git remote URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_remote: Option<String>,

    /// Git commit hash (short)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,

    /// Working directory at creation time
    pub working_dir: String,

    /// Content from nearest .stackstodo-context file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub todo_context_content: Option<String>,

    /// ID of the previous task created in the same shell session
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_prev_task_id: Option<String>,
}

/// YAML frontmatter for a task markdown file.
/// This is the structured data the CLI manages; the body is freeform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFrontmatter {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<Priority>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub due: Option<DateTime<Utc>>,

    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subtask_ids: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<Dependency>,

    pub context: ContextInfo,
}

/// A complete task: frontmatter + freeform markdown body.
#[derive(Debug, Clone)]
pub struct Task {
    pub frontmatter: TaskFrontmatter,
    pub body: String,
}

/// JSON-friendly representation of a task (flat frontmatter + body).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskJson {
    #[serde(flatten)]
    pub frontmatter: TaskFrontmatter,
    pub body: String,
}

impl From<&Task> for TaskJson {
    fn from(task: &Task) -> Self {
        Self {
            frontmatter: task.frontmatter.clone(),
            body: task.body.clone(),
        }
    }
}

impl From<Task> for TaskJson {
    fn from(task: Task) -> Self {
        Self {
            frontmatter: task.frontmatter,
            body: task.body,
        }
    }
}

/// Input format for importing tasks.
#[derive(Debug, Deserialize)]
pub struct TaskImportInput {
    pub title: String,
    pub priority: Option<String>,
    pub tags: Option<Vec<String>>,
    pub stack: Option<String>,
    pub body: Option<String>,
    pub due: Option<String>,
    pub status: Option<String>,
}

impl Task {
    /// Create a new task with sensible defaults.
    pub fn new(id: String, title: String, working_dir: String) -> Self {
        let now = Utc::now();
        Self {
            frontmatter: TaskFrontmatter {
                id,
                title,
                status: TaskStatus::Todo,
                priority: None,
                tags: Vec::new(),
                stack: None,
                due: None,
                created: now,
                modified: now,
                parent_id: None,
                subtask_ids: Vec::new(),
                dependencies: Vec::new(),
                context: ContextInfo {
                    working_dir,
                    ..Default::default()
                },
            },
            body: String::new(),
        }
    }
}
