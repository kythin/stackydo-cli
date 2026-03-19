use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// System-level lifecycle stage. Fixed enum — the system pattern-matches on this
/// for filtering, overdue logic, and hide-archive behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Backlog,
    Active,
    Archive,
}

impl std::fmt::Display for Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Backlog => write!(f, "backlog"),
            Self::Active => write!(f, "active"),
            Self::Archive => write!(f, "archive"),
        }
    }
}

impl std::str::FromStr for Stage {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "backlog" => Ok(Self::Backlog),
            "active" => Ok(Self::Active),
            "archive" => Ok(Self::Archive),
            _ => Err(format!("Invalid stage: {s}. Use: backlog, active, archive")),
        }
    }
}

/// Workspace-configurable workflow mapping from status strings to stages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    /// Maps status string → stage
    pub statuses: BTreeMap<String, Stage>,
    /// Maps alias → canonical status (e.g. "doing" → "in_progress")
    #[serde(default)]
    pub aliases: BTreeMap<String, String>,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        let mut statuses = BTreeMap::new();
        statuses.insert("todo".into(), Stage::Backlog);
        statuses.insert("on_hold".into(), Stage::Backlog);
        statuses.insert("in_progress".into(), Stage::Active);
        statuses.insert("blocked".into(), Stage::Active);
        statuses.insert("in_review".into(), Stage::Active);
        statuses.insert("done".into(), Stage::Archive);
        statuses.insert("cancelled".into(), Stage::Archive);
        // Legacy: treat "deleted" as archive for backward compat
        statuses.insert("deleted".into(), Stage::Archive);

        let mut aliases = BTreeMap::new();
        aliases.insert("doing".into(), "in_progress".into());
        aliases.insert("inprogress".into(), "in_progress".into());
        aliases.insert("canceled".into(), "cancelled".into());

        Self { statuses, aliases }
    }
}

impl WorkflowConfig {
    /// Look up the stage for a status string, falling back to Backlog for unknowns.
    pub fn stage_for(&self, status: &str) -> Stage {
        let canonical = self
            .aliases
            .get(status)
            .map(|s| s.as_str())
            .unwrap_or(status);
        self.statuses
            .get(canonical)
            .copied()
            .unwrap_or(Stage::Backlog)
    }

    /// Resolve aliases and validate a status string. Returns the canonical status.
    /// Rejects "deleted" — delete is a file operation, not a status transition.
    pub fn validate_status(&self, input: &str) -> std::result::Result<String, String> {
        let lower = input.to_lowercase();
        if lower == "deleted" {
            return Err(
                "Cannot set status to 'deleted'. Use the delete command to remove tasks.".into(),
            );
        }
        // Check alias first
        if let Some(canonical) = self.aliases.get(&lower) {
            if self.statuses.contains_key(canonical) {
                return Ok(canonical.clone());
            }
        }
        // Check direct status
        if self.statuses.contains_key(&lower) {
            return Ok(lower);
        }
        let known: Vec<&str> = self
            .statuses
            .keys()
            .filter(|s| s.as_str() != "deleted")
            .map(|s| s.as_str())
            .collect();
        Err(format!(
            "Invalid status: '{input}'. Known statuses: {}",
            known.join(", ")
        ))
    }

    /// Return all statuses that map to the given stage.
    pub fn statuses_for_stage(&self, stage: Stage) -> Vec<&str> {
        self.statuses
            .iter()
            .filter(|(_, &s)| s == stage)
            .map(|(k, _)| k.as_str())
            .collect()
    }

    /// True if the status maps to Archive stage.
    pub fn is_terminal(&self, status: &str) -> bool {
        self.stage_for(status) == Stage::Archive
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
            _ => Err(format!(
                "Invalid priority: {s}. Use: critical, high, medium, low"
            )),
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

    /// Content from nearest stackydo.json context.description field
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

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_id: Option<String>,

    pub title: String,
    pub status: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<Priority>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due: Option<DateTime<Utc>>,

    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subtask_ids: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<Dependency>,

    #[serde(default)]
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

/// JSON-friendly summary of a task (frontmatter only, no body).
/// Used as the default serialization for list/search output.
#[derive(Debug, Clone, Serialize)]
pub struct TaskSummaryJson {
    #[serde(flatten)]
    pub frontmatter: TaskFrontmatter,
}

impl From<&Task> for TaskSummaryJson {
    fn from(task: &Task) -> Self {
        Self {
            frontmatter: task.frontmatter.clone(),
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

#[cfg(test)]
mod workflow_tests {
    use super::*;

    #[test]
    fn stage_for_known_statuses() {
        let wf = WorkflowConfig::default();
        assert_eq!(wf.stage_for("todo"), Stage::Backlog);
        assert_eq!(wf.stage_for("on_hold"), Stage::Backlog);
        assert_eq!(wf.stage_for("in_progress"), Stage::Active);
        assert_eq!(wf.stage_for("blocked"), Stage::Active);
        assert_eq!(wf.stage_for("in_review"), Stage::Active);
        assert_eq!(wf.stage_for("done"), Stage::Archive);
        assert_eq!(wf.stage_for("cancelled"), Stage::Archive);
        assert_eq!(wf.stage_for("deleted"), Stage::Archive);
    }

    #[test]
    fn stage_for_unknown_defaults_to_backlog() {
        let wf = WorkflowConfig::default();
        assert_eq!(wf.stage_for("invented_status"), Stage::Backlog);
        assert_eq!(wf.stage_for(""), Stage::Backlog);
    }

    #[test]
    fn stage_for_resolves_aliases() {
        let wf = WorkflowConfig::default();
        // "doing" → "in_progress" → Active
        assert_eq!(wf.stage_for("doing"), Stage::Active);
        // "canceled" → "cancelled" → Archive
        assert_eq!(wf.stage_for("canceled"), Stage::Archive);
    }

    #[test]
    fn validate_status_accepts_known() {
        let wf = WorkflowConfig::default();
        assert_eq!(wf.validate_status("todo").unwrap(), "todo");
        assert_eq!(wf.validate_status("in_progress").unwrap(), "in_progress");
        assert_eq!(wf.validate_status("done").unwrap(), "done");
        assert_eq!(wf.validate_status("on_hold").unwrap(), "on_hold");
        assert_eq!(wf.validate_status("in_review").unwrap(), "in_review");
    }

    #[test]
    fn validate_status_resolves_aliases() {
        let wf = WorkflowConfig::default();
        assert_eq!(wf.validate_status("doing").unwrap(), "in_progress");
        assert_eq!(wf.validate_status("DOING").unwrap(), "in_progress");
        assert_eq!(wf.validate_status("canceled").unwrap(), "cancelled");
        assert_eq!(wf.validate_status("inprogress").unwrap(), "in_progress");
    }

    #[test]
    fn validate_status_case_insensitive() {
        let wf = WorkflowConfig::default();
        assert_eq!(wf.validate_status("TODO").unwrap(), "todo");
        assert_eq!(wf.validate_status("In_Progress").unwrap(), "in_progress");
        assert_eq!(wf.validate_status("DONE").unwrap(), "done");
    }

    #[test]
    fn validate_status_rejects_unknown() {
        let wf = WorkflowConfig::default();
        assert!(wf.validate_status("invented").is_err());
        assert!(wf.validate_status("").is_err());
    }

    #[test]
    fn validate_status_rejects_deleted() {
        let wf = WorkflowConfig::default();
        let err = wf.validate_status("deleted").unwrap_err();
        assert!(
            err.contains("delete command"),
            "error should mention delete command: {err}"
        );
        // Also rejects case variants
        assert!(wf.validate_status("DELETED").is_err());
    }

    #[test]
    fn is_terminal_checks_archive_stage() {
        let wf = WorkflowConfig::default();
        assert!(wf.is_terminal("done"));
        assert!(wf.is_terminal("cancelled"));
        assert!(wf.is_terminal("deleted"));
        assert!(!wf.is_terminal("todo"));
        assert!(!wf.is_terminal("in_progress"));
        assert!(!wf.is_terminal("blocked"));
        assert!(!wf.is_terminal("on_hold"));
        assert!(!wf.is_terminal("in_review"));
        // Unknown statuses fall back to Backlog, so not terminal
        assert!(!wf.is_terminal("whatever"));
    }

    #[test]
    fn statuses_for_stage_returns_correct_sets() {
        let wf = WorkflowConfig::default();
        let backlog = wf.statuses_for_stage(Stage::Backlog);
        assert!(backlog.contains(&"todo"));
        assert!(backlog.contains(&"on_hold"));
        assert!(!backlog.contains(&"done"));

        let active = wf.statuses_for_stage(Stage::Active);
        assert!(active.contains(&"in_progress"));
        assert!(active.contains(&"blocked"));
        assert!(active.contains(&"in_review"));

        let archive = wf.statuses_for_stage(Stage::Archive);
        assert!(archive.contains(&"done"));
        assert!(archive.contains(&"cancelled"));
    }
}

impl Task {
    /// Create a new task with sensible defaults.
    pub fn new(id: String, title: String, working_dir: String) -> Self {
        let now = Utc::now();
        Self {
            frontmatter: TaskFrontmatter {
                id,
                short_id: None,
                title,
                status: "todo".to_string(),
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
