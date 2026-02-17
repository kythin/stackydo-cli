use rmcp::{handler::server::wrapper::Parameters, schemars, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::commands::show::resolve_task_pub;
use crate::commands::util::parse_due_date;
use crate::context::dir_context;
use crate::model::task::{Priority, Task, TaskJson, TaskStatus};
use crate::storage::manifest_store::ManifestStore;
use crate::storage::task_store::TaskStore;
use chrono::Utc;
use std::collections::BTreeMap;

use super::StackstodoMcp;

pub fn create_tool_router() -> rmcp::handler::server::router::tool::ToolRouter<StackstodoMcp> {
    StackstodoMcp::tool_router()
}

// ── Parameter structs ──

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListTasksParams {
    /// Filter by status: todo, in_progress, done, blocked, cancelled
    #[schemars(default)]
    pub status: Option<String>,
    /// Filter by tag name
    #[schemars(default)]
    pub tag: Option<String>,
    /// Filter by priority: critical, high, medium, low
    #[schemars(default)]
    pub priority: Option<String>,
    /// Filter by stack name
    #[schemars(default)]
    pub stack: Option<String>,
    /// Sort by: created (default), due, modified, priority
    #[schemars(default)]
    pub sort: Option<String>,
    /// Maximum number of tasks to return
    #[schemars(default)]
    pub limit: Option<usize>,
    /// Only show overdue tasks (due date passed, not done/cancelled)
    #[schemars(default)]
    pub overdue: Option<bool>,
    /// Only show tasks due before this date (YYYY-MM-DD)
    #[schemars(default)]
    pub due_before: Option<String>,
    /// Only show tasks due after this date (YYYY-MM-DD)
    #[schemars(default)]
    pub due_after: Option<String>,
    /// Group results by field (supported: "stack")
    #[schemars(default)]
    pub group_by: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetTaskParams {
    /// Task ID or unique prefix
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateTaskParams {
    /// Task title (required)
    pub title: String,
    /// Priority: critical, high, medium, low
    #[schemars(default)]
    pub priority: Option<String>,
    /// Comma-separated tags
    #[schemars(default)]
    pub tags: Option<String>,
    /// Stack name (workstream organizer)
    #[schemars(default)]
    pub stack: Option<String>,
    /// Task body (freeform markdown)
    #[schemars(default)]
    pub body: Option<String>,
    /// Due date (YYYY-MM-DD or YYYY-MM-DD HH:MM)
    #[schemars(default)]
    pub due: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateTaskParams {
    /// Task ID or unique prefix
    pub id: String,
    /// New title
    #[schemars(default)]
    pub title: Option<String>,
    /// New status: todo, in_progress, done, blocked, cancelled
    #[schemars(default)]
    pub status: Option<String>,
    /// New priority: critical, high, medium, low (or "none" to clear)
    #[schemars(default)]
    pub priority: Option<String>,
    /// New tags (comma-separated, replaces existing; empty string clears)
    #[schemars(default)]
    pub tags: Option<String>,
    /// New stack (empty string clears)
    #[schemars(default)]
    pub stack: Option<String>,
    /// New due date (empty string clears)
    #[schemars(default)]
    pub due: Option<String>,
    /// Append a timestamped note to the body
    #[schemars(default)]
    pub note: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CompleteTaskParams {
    /// Task ID or unique prefix
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteTaskParams {
    /// Task ID or unique prefix
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchTasksParams {
    /// Search query (matches against title and body, case-insensitive)
    pub query: String,
}

// ── Helper for error conversion ──

fn err_to_string(e: impl std::fmt::Display) -> String {
    format!("Error: {e}")
}

// ── Tool implementations ──

#[tool_router]
impl StackstodoMcp {
    #[rmcp::tool(description = "List tasks with optional filters and sorting. Returns JSON array of tasks, or grouped object when group_by is specified.")]
    fn list_tasks(
        &self,
        Parameters(params): Parameters<ListTasksParams>,
    ) -> String {
        let store = TaskStore::new();
        let mut tasks = match store.load_all() {
            Ok(t) => t,
            Err(e) => return err_to_string(e),
        };

        // Filter by status
        if let Some(ref status_str) = params.status {
            match status_str.parse::<TaskStatus>() {
                Ok(s) => tasks.retain(|t| t.frontmatter.status == s),
                Err(e) => return err_to_string(e),
            }
        }

        // Filter by tag
        if let Some(ref tag) = params.tag {
            let tag_lower = tag.to_lowercase();
            tasks.retain(|t| {
                t.frontmatter
                    .tags
                    .iter()
                    .any(|tt| tt.to_lowercase() == tag_lower)
            });
        }

        // Filter by priority
        if let Some(ref pri_str) = params.priority {
            match pri_str.parse::<Priority>() {
                Ok(pri) => tasks.retain(|t| t.frontmatter.priority.as_ref() == Some(&pri)),
                Err(e) => return err_to_string(e),
            }
        }

        // Filter by stack
        if let Some(ref stack) = params.stack {
            let stack_lower = stack.to_lowercase();
            tasks.retain(|t| {
                t.frontmatter
                    .stack
                    .as_ref()
                    .map(|s| s.to_lowercase() == stack_lower)
                    .unwrap_or(false)
            });
        }

        // Filter: overdue
        if params.overdue.unwrap_or(false) {
            let now = Utc::now();
            tasks.retain(|t| {
                if let Some(due) = t.frontmatter.due {
                    due < now
                        && t.frontmatter.status != TaskStatus::Done
                        && t.frontmatter.status != TaskStatus::Cancelled
                } else {
                    false
                }
            });
        }

        // Filter: due_before
        if let Some(ref date_str) = params.due_before {
            match parse_due_date(date_str) {
                Ok(cutoff) => tasks.retain(|t| t.frontmatter.due.map(|d| d < cutoff).unwrap_or(false)),
                Err(e) => return err_to_string(e),
            }
        }

        // Filter: due_after
        if let Some(ref date_str) = params.due_after {
            match parse_due_date(date_str) {
                Ok(cutoff) => tasks.retain(|t| t.frontmatter.due.map(|d| d > cutoff).unwrap_or(false)),
                Err(e) => return err_to_string(e),
            }
        }

        // Sort
        let sort_field = params.sort.as_deref().unwrap_or("created");
        match sort_field {
            "due" => tasks.sort_by(|a, b| a.frontmatter.due.cmp(&b.frontmatter.due)),
            "modified" => tasks.sort_by(|a, b| b.frontmatter.modified.cmp(&a.frontmatter.modified)),
            "priority" => tasks.sort_by(|a, b| a.frontmatter.priority.cmp(&b.frontmatter.priority)),
            _ => tasks.sort_by(|a, b| b.frontmatter.created.cmp(&a.frontmatter.created)),
        }

        // Limit
        if let Some(limit) = params.limit {
            tasks.truncate(limit);
        }

        // Group-by
        if let Some(ref group_field) = params.group_by {
            if group_field == "stack" {
                let mut groups: BTreeMap<String, Vec<TaskJson>> = BTreeMap::new();
                for task in &tasks {
                    let key = task
                        .frontmatter
                        .stack
                        .clone()
                        .unwrap_or_else(|| "(no stack)".to_string());
                    groups.entry(key).or_default().push(TaskJson::from(task));
                }
                return serde_json::to_string(&groups).unwrap_or_else(err_to_string);
            }
            return err_to_string(format!("Unknown group_by field: '{group_field}'. Supported: stack"));
        }

        let json_tasks: Vec<TaskJson> = tasks.iter().map(TaskJson::from).collect();
        serde_json::to_string(&json_tasks).unwrap_or_else(err_to_string)
    }

    #[rmcp::tool(description = "Get a single task by ID (supports prefix matching). Returns full task JSON including body and context.")]
    fn get_task(&self, Parameters(params): Parameters<GetTaskParams>) -> String {
        let store = TaskStore::new();
        match resolve_task_pub(&store, &params.id) {
            Ok(task) => {
                let json_task = TaskJson::from(&task);
                serde_json::to_string(&json_task).unwrap_or_else(err_to_string)
            }
            Err(e) => err_to_string(e),
        }
    }

    #[rmcp::tool(description = "Create a new task. Returns the new task's ULID on success.")]
    fn create_task(
        &self,
        Parameters(params): Parameters<CreateTaskParams>,
    ) -> String {
        let store = TaskStore::new();
        let manifest_store = ManifestStore::new();
        let id = ulid::Ulid::new().to_string();

        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| ".".into());

        let context_path = std::env::current_dir().unwrap_or_else(|_| ".".into());
        let ctx = dir_context::capture(&context_path);

        let mut task = Task::new(id.clone(), params.title, cwd);
        task.frontmatter.context = ctx;

        if let Some(body) = params.body {
            task.body = body;
        }

        if let Some(ref pri_str) = params.priority {
            match pri_str.parse::<Priority>() {
                Ok(p) => task.frontmatter.priority = Some(p),
                Err(e) => return err_to_string(e),
            }
        }

        if let Some(ref tags_csv) = params.tags {
            let tags: Vec<String> = tags_csv
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let _ = manifest_store.register_tags(&tags);
            task.frontmatter.tags = tags;
        }

        if let Some(ref stack) = params.stack {
            let stack = stack.trim().to_string();
            if !stack.is_empty() {
                let _ = manifest_store.register_stack(&stack);
                task.frontmatter.stack = Some(stack);
            }
        }

        if let Some(ref due_str) = params.due {
            match parse_due_date(due_str) {
                Ok(dt) => task.frontmatter.due = Some(dt),
                Err(e) => return err_to_string(e),
            }
        }

        match store.save(&task) {
            Ok(()) => id,
            Err(e) => err_to_string(e),
        }
    }

    #[rmcp::tool(description = "Update an existing task. Returns confirmation and updated task JSON. Use --note to append timestamped progress notes.")]
    fn update_task(
        &self,
        Parameters(params): Parameters<UpdateTaskParams>,
    ) -> String {
        let store = TaskStore::new();
        let manifest_store = ManifestStore::new();
        let mut task = match resolve_task_pub(&store, &params.id) {
            Ok(t) => t,
            Err(e) => return err_to_string(e),
        };

        let mut changed = false;

        if let Some(ref title) = params.title {
            task.frontmatter.title = title.clone();
            changed = true;
        }

        if let Some(ref status_str) = params.status {
            match status_str.parse::<TaskStatus>() {
                Ok(s) => {
                    task.frontmatter.status = s;
                    changed = true;
                }
                Err(e) => return err_to_string(e),
            }
        }

        if let Some(ref pri_str) = params.priority {
            if pri_str.eq_ignore_ascii_case("none") || pri_str.is_empty() {
                task.frontmatter.priority = None;
            } else {
                match pri_str.parse::<Priority>() {
                    Ok(p) => task.frontmatter.priority = Some(p),
                    Err(e) => return err_to_string(e),
                }
            }
            changed = true;
        }

        if let Some(ref tags_csv) = params.tags {
            if tags_csv.is_empty() {
                task.frontmatter.tags = Vec::new();
            } else {
                let tags: Vec<String> = tags_csv
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                let _ = manifest_store.register_tags(&tags);
                task.frontmatter.tags = tags;
            }
            changed = true;
        }

        if let Some(ref stack) = params.stack {
            let stack = stack.trim().to_string();
            if stack.is_empty() {
                task.frontmatter.stack = None;
            } else {
                let _ = manifest_store.register_stack(&stack);
                task.frontmatter.stack = Some(stack);
            }
            changed = true;
        }

        if let Some(ref due_str) = params.due {
            if due_str.is_empty() {
                task.frontmatter.due = None;
            } else {
                match parse_due_date(due_str) {
                    Ok(dt) => task.frontmatter.due = Some(dt),
                    Err(e) => return err_to_string(e),
                }
            }
            changed = true;
        }

        if let Some(ref note_text) = params.note {
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M");
            let entry = format!("\n[{timestamp}] {note_text}");
            if task.body.is_empty() {
                task.body = entry.trim_start().to_string();
            } else {
                task.body.push_str(&entry);
            }
            changed = true;
        }

        if !changed {
            return "No changes specified.".to_string();
        }

        task.frontmatter.modified = Utc::now();
        match store.save(&task) {
            Ok(()) => {
                let json_task = TaskJson::from(&task);
                serde_json::to_string(&json_task).unwrap_or_else(err_to_string)
            }
            Err(e) => err_to_string(e),
        }
    }

    #[rmcp::tool(description = "Mark a task as done.")]
    fn complete_task(
        &self,
        Parameters(params): Parameters<CompleteTaskParams>,
    ) -> String {
        let store = TaskStore::new();
        let mut task = match resolve_task_pub(&store, &params.id) {
            Ok(t) => t,
            Err(e) => return err_to_string(e),
        };

        task.frontmatter.status = TaskStatus::Done;
        task.frontmatter.modified = Utc::now();

        match store.save(&task) {
            Ok(()) => format!(
                "Completed: {} — {}",
                &task.frontmatter.id[..10],
                task.frontmatter.title
            ),
            Err(e) => err_to_string(e),
        }
    }

    #[rmcp::tool(description = "Delete a task permanently.")]
    fn delete_task(
        &self,
        Parameters(params): Parameters<DeleteTaskParams>,
    ) -> String {
        let store = TaskStore::new();
        let task = match resolve_task_pub(&store, &params.id) {
            Ok(t) => t,
            Err(e) => return err_to_string(e),
        };

        let task_id = task.frontmatter.id.clone();
        let title = task.frontmatter.title.clone();

        // Clear parent's subtask reference
        if let Some(ref parent_id) = task.frontmatter.parent_id {
            if let Ok(mut parent) = store.load(parent_id) {
                parent.frontmatter.subtask_ids.retain(|s| s != &task_id);
                parent.frontmatter.modified = Utc::now();
                let _ = store.save(&parent);
            }
        }

        match store.delete(&task_id) {
            Ok(()) => format!("Deleted: {} — {}", &task_id[..10], title),
            Err(e) => err_to_string(e),
        }
    }

    #[rmcp::tool(description = "Search tasks by matching query against title and body (case-insensitive). Returns JSON array of matching tasks.")]
    fn search_tasks(
        &self,
        Parameters(params): Parameters<SearchTasksParams>,
    ) -> String {
        let store = TaskStore::new();
        match store.search(&params.query) {
            Ok(tasks) => {
                let json_tasks: Vec<TaskJson> = tasks.iter().map(TaskJson::from).collect();
                serde_json::to_string(&json_tasks).unwrap_or_else(err_to_string)
            }
            Err(e) => err_to_string(e),
        }
    }

    #[rmcp::tool(description = "Get summary statistics: total tasks, overdue count, breakdown by status/stack/tag.")]
    fn get_stats(&self) -> String {
        let store = TaskStore::new();
        let tasks = match store.load_all() {
            Ok(t) => t,
            Err(e) => return err_to_string(e),
        };
        let now = Utc::now();

        let total = tasks.len();
        let mut by_status: BTreeMap<String, usize> = BTreeMap::new();
        let mut by_stack: BTreeMap<String, StackStatsJson> = BTreeMap::new();
        let mut tags: BTreeMap<String, usize> = BTreeMap::new();
        let mut overdue = 0usize;

        for task in &tasks {
            let status_str = task.frontmatter.status.to_string();
            *by_status.entry(status_str.clone()).or_default() += 1;

            if let Some(due) = task.frontmatter.due {
                if due < now
                    && task.frontmatter.status != TaskStatus::Done
                    && task.frontmatter.status != TaskStatus::Cancelled
                {
                    overdue += 1;
                }
            }

            let stack_name = task
                .frontmatter
                .stack
                .clone()
                .unwrap_or_else(|| "(no stack)".to_string());
            let stack_entry = by_stack
                .entry(stack_name)
                .or_insert_with(|| StackStatsJson {
                    total: 0,
                    by_status: BTreeMap::new(),
                });
            stack_entry.total += 1;
            *stack_entry.by_status.entry(status_str).or_default() += 1;

            for tag in &task.frontmatter.tags {
                *tags.entry(tag.clone()).or_default() += 1;
            }
        }

        let output = StatsJson {
            total,
            overdue,
            by_status,
            by_stack,
            tags,
        };
        serde_json::to_string(&output).unwrap_or_else(err_to_string)
    }

    #[rmcp::tool(description = "Get all stacks with per-stack task counts and status breakdowns.")]
    fn get_stacks(&self) -> String {
        let store = TaskStore::new();
        let manifest_store = ManifestStore::new();
        let tasks = match store.load_all() {
            Ok(t) => t,
            Err(e) => return err_to_string(e),
        };
        let manifest = match manifest_store.load() {
            Ok(m) => m,
            Err(e) => return err_to_string(e),
        };

        let mut all_stacks: std::collections::BTreeSet<String> =
            manifest.stacks.iter().cloned().collect();
        for task in &tasks {
            if let Some(ref stack) = task.frontmatter.stack {
                all_stacks.insert(stack.clone());
            }
        }

        let mut stack_infos: BTreeMap<String, StackStatsJson> = BTreeMap::new();
        for stack_name in &all_stacks {
            stack_infos.insert(
                stack_name.clone(),
                StackStatsJson {
                    total: 0,
                    by_status: BTreeMap::new(),
                },
            );
        }

        for task in &tasks {
            if let Some(ref stack) = task.frontmatter.stack {
                if let Some(info) = stack_infos.get_mut(stack) {
                    info.total += 1;
                    let status_str = task.frontmatter.status.to_string();
                    *info.by_status.entry(status_str).or_default() += 1;
                }
            }
        }

        serde_json::to_string(&stack_infos).unwrap_or_else(err_to_string)
    }
}

// ── JSON output types (internal to MCP tools) ──

#[derive(Debug, Serialize)]
struct StackStatsJson {
    total: usize,
    by_status: BTreeMap<String, usize>,
}

#[derive(Debug, Serialize)]
struct StatsJson {
    total: usize,
    overdue: usize,
    by_status: BTreeMap<String, usize>,
    by_stack: BTreeMap<String, StackStatsJson>,
    tags: BTreeMap<String, usize>,
}
