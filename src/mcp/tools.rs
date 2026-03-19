use rmcp::{handler::server::wrapper::Parameters, schemars, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::commands::show::resolve_task_pub;
use crate::commands::util::{
    active_workflow, apply_filters, apply_pagination, apply_sort, display_id, effective_limit,
    parse_due_date, FilterParams,
};
use crate::context::dir_context;
use crate::model::task::{Priority, Task, TaskJson, TaskSummaryJson};
use crate::storage::manifest_store::ManifestStore;
use crate::storage::task_store::TaskStore;
use crate::storage::workspace;
use chrono::Utc;
use std::collections::BTreeMap;

use super::StackydoMcp;

pub fn create_tool_router() -> rmcp::handler::server::router::tool::ToolRouter<StackydoMcp> {
    StackydoMcp::tool_router()
}

// ── Parameter structs ──

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListTasksParams {
    /// Filter by status: todo, in_progress, done, blocked, cancelled, on_hold, in_review
    #[schemars(default)]
    pub status: Option<String>,
    /// Filter by stage: backlog, active, archive
    #[schemars(default)]
    pub stage: Option<String>,
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
    /// Max results to return (default: 50, 0 = no limit)
    #[schemars(default)]
    pub limit: Option<usize>,
    /// Skip the first N results (0-indexed, default: 0). E.g. offset=50 + limit=50 returns results 51-100
    #[schemars(default)]
    pub offset: Option<usize>,
    /// Only show overdue tasks (due date passed, not in archive stage)
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
    /// Include full task body in output (default: false, returns frontmatter only)
    #[schemars(default)]
    pub full: Option<bool>,
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
    /// New status: todo, in_progress, done, blocked, cancelled, on_hold, in_review
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
    /// Replace entire body content (empty string clears body)
    #[schemars(default)]
    pub body_replace: Option<String>,
    /// Sed-style substitution on body: s/pattern/replacement/[g]
    #[schemars(default)]
    pub body_sub: Option<String>,
    /// Append text to body
    #[schemars(default)]
    pub body_append: Option<String>,
    /// Append a timestamped note to the body
    #[schemars(default)]
    pub note: Option<String>,
    /// Preview resulting body without saving (returns {"preview": true, "body": "..."})
    #[schemars(default)]
    pub dry_run: Option<bool>,
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
    /// Filter results by status: todo, in_progress, done, blocked, cancelled, on_hold, in_review
    #[schemars(default)]
    pub status: Option<String>,
    /// Filter results by stage: backlog, active, archive
    #[schemars(default)]
    pub stage: Option<String>,
    /// Filter results by tag name
    #[schemars(default)]
    pub tag: Option<String>,
    /// Filter results by priority: critical, high, medium, low
    #[schemars(default)]
    pub priority: Option<String>,
    /// Filter results by stack name
    #[schemars(default)]
    pub stack: Option<String>,
    /// Sort by: created (default), due, modified, priority
    #[schemars(default)]
    pub sort: Option<String>,
    /// Max results to return (default: 50, 0 = no limit)
    #[schemars(default)]
    pub limit: Option<usize>,
    /// Skip the first N results (0-indexed, default: 0)
    #[schemars(default)]
    pub offset: Option<usize>,
    /// Only show overdue results (due date passed, not in archive stage)
    #[schemars(default)]
    pub overdue: Option<bool>,
    /// Only show results due before this date (YYYY-MM-DD)
    #[schemars(default)]
    pub due_before: Option<String>,
    /// Only show results due after this date (YYYY-MM-DD)
    #[schemars(default)]
    pub due_after: Option<String>,
    /// Group results by field (supported: "stack")
    #[schemars(default)]
    pub group_by: Option<String>,
    /// Include full task body in output (default: false, returns frontmatter only)
    #[schemars(default)]
    pub full: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListWorkspacesParams {
    // No params needed — discovery is automatic
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MigrateTasksParams {
    /// Source workspace path (directory, stackydo.json, or store dir)
    pub source: String,
    /// Destination workspace path
    pub dest: String,
    /// Filter by stack name(s), comma-separated
    #[schemars(default)]
    pub stack: Option<String>,
    /// Specific task ID or prefix (comma-separated for multiple)
    #[schemars(default)]
    pub task: Option<String>,
    /// Select all tasks from matched stacks
    #[schemars(default)]
    pub all: Option<bool>,
    /// Operation: "move" or "copy" (default: "copy")
    #[schemars(default)]
    pub operation: Option<String>,
    /// Preview only — don't make changes
    #[schemars(default)]
    pub dry_run: Option<bool>,
    /// Overwrite conflicting task IDs in destination
    #[schemars(default)]
    pub force: Option<bool>,
}

// ── Helper for error conversion ──

fn err_to_string(e: impl std::fmt::Display) -> String {
    format!("Error: {e}")
}

// ── Tool implementations ──

#[tool_router]
impl StackydoMcp {
    #[rmcp::tool(
        description = "List tasks with optional filters and sorting. Archive-stage tasks (done, cancelled) are hidden by default; pass status or stage to include them. Returns JSON array of tasks (frontmatter only by default; set full=true to include body). Default limit: 50 (set limit=0 for no limit). Use offset for pagination."
    )]
    fn list_tasks(&self, Parameters(params): Parameters<ListTasksParams>) -> String {
        let store = TaskStore::new();
        let mut tasks = match store.load_all() {
            Ok(t) => t,
            Err(e) => return err_to_string(e),
        };

        // Hide archive-stage tasks by default unless filtering by status or stage
        if params.status.is_none() && params.stage.is_none() {
            let workflow = active_workflow();
            tasks.retain(|t| !workflow.is_terminal(&t.frontmatter.status));
        }

        // Apply filters
        if let Err(e) = apply_filters(
            &mut tasks,
            &FilterParams {
                status: params.status.as_deref(),
                stage: params.stage.as_deref(),
                tag: params.tag.as_deref(),
                priority: params.priority.as_deref(),
                stack: params.stack.as_deref(),
                overdue: params.overdue.unwrap_or(false),
                due_before: params.due_before.as_deref(),
                due_after: params.due_after.as_deref(),
                due_this_week: false,
            },
        ) {
            return err_to_string(e);
        }

        // Sort
        if let Err(e) = apply_sort(
            &mut tasks,
            params.sort.as_deref().unwrap_or("created"),
            false,
        ) {
            return err_to_string(e);
        }

        // Pagination
        let limit = effective_limit(params.limit);
        let offset = params.offset.unwrap_or(0);
        apply_pagination(&mut tasks, offset, limit);

        let full = params.full.unwrap_or(false);

        // Group-by
        if let Some(ref group_field) = params.group_by {
            if group_field == "stack" {
                if full {
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
                } else {
                    let mut groups: BTreeMap<String, Vec<TaskSummaryJson>> = BTreeMap::new();
                    for task in &tasks {
                        let key = task
                            .frontmatter
                            .stack
                            .clone()
                            .unwrap_or_else(|| "(no stack)".to_string());
                        groups
                            .entry(key)
                            .or_default()
                            .push(TaskSummaryJson::from(task));
                    }
                    return serde_json::to_string(&groups).unwrap_or_else(err_to_string);
                }
            }
            return err_to_string(format!(
                "Unknown group_by field: '{group_field}'. Supported: stack"
            ));
        }

        if full {
            let json_tasks: Vec<TaskJson> = tasks.iter().map(TaskJson::from).collect();
            serde_json::to_string(&json_tasks).unwrap_or_else(err_to_string)
        } else {
            let json_tasks: Vec<TaskSummaryJson> =
                tasks.iter().map(TaskSummaryJson::from).collect();
            serde_json::to_string(&json_tasks).unwrap_or_else(err_to_string)
        }
    }

    #[rmcp::tool(
        description = "Get a single task by ID (supports prefix matching). Returns full task JSON including body and context."
    )]
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
    fn create_task(&self, Parameters(params): Parameters<CreateTaskParams>) -> String {
        let store = TaskStore::new();
        let manifest_store = ManifestStore::new();
        let id = ulid::Ulid::new().to_string();

        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| ".".into());

        let context_path = std::env::current_dir().unwrap_or_else(|_| ".".into());
        let ctx = dir_context::capture(&context_path);

        // Validate title
        let title = params.title.trim().to_string();
        if title.is_empty() {
            return err_to_string("Title cannot be empty");
        }

        let mut task = Task::new(id.clone(), title, cwd);
        task.frontmatter.context = ctx;

        // Assign short ID
        if let Ok(sid) = manifest_store.allocate_short_id() {
            task.frontmatter.short_id = Some(sid);
        }

        if let Some(body) = params.body {
            // Convert literal \n escape sequences from MCP JSON to real newlines
            task.body = body.replace("\\n", "\n");
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
            Ok(()) => {
                let result = serde_json::json!({
                    "id": id,
                    "short_id": task.frontmatter.short_id,
                });
                serde_json::to_string(&result).unwrap_or(id)
            }
            Err(e) => err_to_string(e),
        }
    }

    #[rmcp::tool(
        description = "Update an existing task. Returns confirmation and updated task JSON. Body operations apply in order: body_replace (set body), body_sub (sed-style s/pat/repl/[g] substitution), body_append (append text), note (timestamped append). Use dry_run=true to preview the resulting body without saving."
    )]
    fn update_task(&self, Parameters(params): Parameters<UpdateTaskParams>) -> String {
        let store = TaskStore::new();
        let manifest_store = ManifestStore::new();
        let mut task = match resolve_task_pub(&store, &params.id) {
            Ok(t) => t,
            Err(e) => return err_to_string(e),
        };

        let mut changed = false;

        if let Some(ref title) = params.title {
            let title = title.trim().to_string();
            if title.is_empty() {
                return err_to_string("Title cannot be empty");
            }
            task.frontmatter.title = title;
            changed = true;
        }

        if let Some(ref status_str) = params.status {
            let workflow = active_workflow();
            match workflow.validate_status(status_str) {
                Ok(canonical) => {
                    task.frontmatter.status = canonical;
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

        // Body replace (step 1)
        if let Some(ref new_body) = params.body_replace {
            task.body = new_body.replace("\\n", "\n");
            changed = true;
        }

        // Body substitution (step 2)
        if let Some(ref expr) = params.body_sub {
            match crate::commands::body_edit::parse_sed_expression(expr) {
                Ok((regex, replacement, global)) => {
                    task.body = crate::commands::body_edit::apply_substitution(
                        &task.body,
                        &regex,
                        &replacement,
                        global,
                    );
                    changed = true;
                }
                Err(e) => return err_to_string(e),
            }
        }

        // Body append (step 3)
        if let Some(ref append_text) = params.body_append {
            let append_text = append_text.replace("\\n", "\n");
            if task.body.is_empty() {
                task.body = append_text;
            } else {
                task.body.push('\n');
                task.body.push_str(&append_text);
            }
            changed = true;
        }

        // Note — timestamped append (step 4)
        if let Some(ref note_text) = params.note {
            // Convert literal \n escape sequences from MCP JSON to real newlines
            let note_text = note_text.replace("\\n", "\n");
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

        // Dry-run: return preview without saving
        if params.dry_run.unwrap_or(false) {
            let has_body_op = params.body_replace.is_some()
                || params.body_sub.is_some()
                || params.body_append.is_some()
                || params.note.is_some();
            if !has_body_op {
                return err_to_string("dry_run requires a body operation (body_replace, body_sub, body_append, or note)");
            }
            let preview = serde_json::json!({
                "preview": true,
                "body": task.body,
            });
            return serde_json::to_string(&preview).unwrap_or_else(err_to_string);
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
    fn complete_task(&self, Parameters(params): Parameters<CompleteTaskParams>) -> String {
        let store = TaskStore::new();
        let mut task = match resolve_task_pub(&store, &params.id) {
            Ok(t) => t,
            Err(e) => return err_to_string(e),
        };

        task.frontmatter.status = "done".to_string();
        task.frontmatter.modified = Utc::now();

        match store.save(&task) {
            Ok(()) => format!(
                "Completed: {} — {}",
                display_id(&task.frontmatter),
                task.frontmatter.title
            ),
            Err(e) => err_to_string(e),
        }
    }

    #[rmcp::tool(description = "Delete a task permanently (removes the file).")]
    fn delete_task(&self, Parameters(params): Parameters<DeleteTaskParams>) -> String {
        let store = TaskStore::new();
        let manifest_store = ManifestStore::new();

        let task = match resolve_task_pub(&store, &params.id) {
            Ok(t) => t,
            Err(e) => return err_to_string(e),
        };

        let task_id = task.frontmatter.id.clone();
        let title = task.frontmatter.title.clone();
        let did = display_id(&task.frontmatter).to_string();

        // Clear parent's subtask reference
        if let Some(ref parent_id) = task.frontmatter.parent_id {
            if let Ok(mut parent) = store.load(parent_id) {
                parent.frontmatter.subtask_ids.retain(|s| s != &task_id);
                parent.frontmatter.modified = Utc::now();
                let _ = store.save(&parent);
            }
        }

        match store.delete(&task_id) {
            Ok(()) => {
                if let Ok(remaining) = store.load_all() {
                    let _ = manifest_store.prune_stacks_and_tags(&remaining);
                }
                format!("Deleted: {did} — {title}")
            }
            Err(e) => err_to_string(e),
        }
    }

    #[rmcp::tool(
        description = "Search tasks by matching query against title and body (case-insensitive). Archive-stage tasks (done, cancelled) are hidden by default; pass status or stage to include them. Returns JSON array of matching tasks (frontmatter only by default; set full=true to include body). Default limit: 50 (set limit=0 for no limit). Supports filtering, sorting, and pagination."
    )]
    fn search_tasks(&self, Parameters(params): Parameters<SearchTasksParams>) -> String {
        let store = TaskStore::new();
        let mut tasks = match store.search(&params.query) {
            Ok(t) => t,
            Err(e) => return err_to_string(e),
        };

        // Hide archive-stage tasks by default unless filtering by status or stage
        if params.status.is_none() && params.stage.is_none() {
            let workflow = active_workflow();
            tasks.retain(|t| !workflow.is_terminal(&t.frontmatter.status));
        }

        // Apply filters
        if let Err(e) = apply_filters(
            &mut tasks,
            &FilterParams {
                status: params.status.as_deref(),
                stage: params.stage.as_deref(),
                tag: params.tag.as_deref(),
                priority: params.priority.as_deref(),
                stack: params.stack.as_deref(),
                overdue: params.overdue.unwrap_or(false),
                due_before: params.due_before.as_deref(),
                due_after: params.due_after.as_deref(),
                due_this_week: false,
            },
        ) {
            return err_to_string(e);
        }

        // Sort
        if let Err(e) = apply_sort(
            &mut tasks,
            params.sort.as_deref().unwrap_or("created"),
            false,
        ) {
            return err_to_string(e);
        }

        // Pagination
        let limit = effective_limit(params.limit);
        let offset = params.offset.unwrap_or(0);
        apply_pagination(&mut tasks, offset, limit);

        let full = params.full.unwrap_or(false);

        // Group-by
        if let Some(ref group_field) = params.group_by {
            if group_field == "stack" {
                if full {
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
                } else {
                    let mut groups: BTreeMap<String, Vec<TaskSummaryJson>> = BTreeMap::new();
                    for task in &tasks {
                        let key = task
                            .frontmatter
                            .stack
                            .clone()
                            .unwrap_or_else(|| "(no stack)".to_string());
                        groups
                            .entry(key)
                            .or_default()
                            .push(TaskSummaryJson::from(task));
                    }
                    return serde_json::to_string(&groups).unwrap_or_else(err_to_string);
                }
            }
            return err_to_string(format!(
                "Unknown group_by field: '{group_field}'. Supported: stack"
            ));
        }

        if full {
            let json_tasks: Vec<TaskJson> = tasks.iter().map(TaskJson::from).collect();
            serde_json::to_string(&json_tasks).unwrap_or_else(err_to_string)
        } else {
            let json_tasks: Vec<TaskSummaryJson> =
                tasks.iter().map(TaskSummaryJson::from).collect();
            serde_json::to_string(&json_tasks).unwrap_or_else(err_to_string)
        }
    }

    #[rmcp::tool(
        description = "Get summary statistics: total tasks, overdue count, breakdown by status/stage/stack/tag."
    )]
    fn get_stats(&self) -> String {
        let store = TaskStore::new();
        let tasks = match store.load_all() {
            Ok(t) => t,
            Err(e) => return err_to_string(e),
        };
        let workflow = active_workflow();
        let now = Utc::now();

        let total = tasks.len();
        let mut by_status: BTreeMap<String, usize> = BTreeMap::new();
        let mut by_stage: BTreeMap<String, usize> = BTreeMap::new();
        let mut by_stack: BTreeMap<String, StackStatsJson> = BTreeMap::new();
        let mut tags: BTreeMap<String, usize> = BTreeMap::new();
        let mut overdue = 0usize;

        for task in &tasks {
            let status_str = task.frontmatter.status.clone();
            *by_status.entry(status_str.clone()).or_default() += 1;

            let stage = workflow.stage_for(&status_str);
            *by_stage.entry(stage.to_string()).or_default() += 1;

            if let Some(due) = task.frontmatter.due {
                if due < now && !workflow.is_terminal(&status_str) {
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
            by_stage,
            by_stack,
            tags,
        };
        serde_json::to_string(&output).unwrap_or_else(err_to_string)
    }

    #[rmcp::tool(
        description = "Discover and list all stackydo workspaces on the system. Returns JSON array of workspace info including store path, task count, stacks, and project name."
    )]
    fn list_workspaces(
        &self,
        #[allow(unused_variables)] Parameters(params): Parameters<ListWorkspacesParams>,
    ) -> String {
        let workspaces = workspace::discover_workspaces();
        serde_json::to_string(&workspaces).unwrap_or_else(err_to_string)
    }

    #[rmcp::tool(
        description = "Move or copy tasks between workspaces. Requires source and dest paths. Use dry_run to preview. Returns summary of migrated tasks."
    )]
    fn migrate_tasks(&self, Parameters(params): Parameters<MigrateTasksParams>) -> String {
        let source_dir = match workspace::resolve_workspace_path(&params.source) {
            Ok(d) => d,
            Err(e) => return err_to_string(format!("Invalid source: {e}")),
        };
        let dest_dir = match workspace::resolve_workspace_path(&params.dest) {
            Ok(d) => d,
            Err(e) => return err_to_string(format!("Invalid dest: {e}")),
        };

        let source_store = TaskStore::with_root(source_dir.clone());
        let all_tasks = match source_store.load_all() {
            Ok(t) => t,
            Err(e) => return err_to_string(e),
        };

        // Parse filters
        let task_ids: Vec<String> = params
            .task
            .as_deref()
            .unwrap_or("")
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let stacks: Vec<String> = params
            .stack
            .as_deref()
            .unwrap_or("")
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let select_all = params.all.unwrap_or(false);

        if task_ids.is_empty() && stacks.is_empty() && !select_all {
            return err_to_string("Specify task, stack, or all=true to select tasks");
        }

        // Select tasks
        let selected: Vec<&Task> = if select_all && stacks.is_empty() && task_ids.is_empty() {
            all_tasks.iter().collect()
        } else {
            let mut sel = Vec::new();
            let stack_set: std::collections::HashSet<&str> =
                stacks.iter().map(|s| s.as_str()).collect();
            for task in &all_tasks {
                let by_id = task_ids
                    .iter()
                    .any(|prefix| task.frontmatter.id.starts_with(prefix));
                let by_stack = task
                    .frontmatter
                    .stack
                    .as_deref()
                    .map(|s| stack_set.contains(s))
                    .unwrap_or(false);
                if by_id || by_stack {
                    sel.push(task);
                }
            }
            sel
        };

        if selected.is_empty() {
            return "No tasks matched the given filters.".to_string();
        }

        let is_move = params.operation.as_deref() == Some("move");
        let dry_run = params.dry_run.unwrap_or(false);
        let force = params.force.unwrap_or(false);
        let op_str = if is_move { "Move" } else { "Copy" };
        let op_past = if is_move { "Moved" } else { "Copied" };

        // Check for conflicts
        let dest_store = TaskStore::with_root(dest_dir.clone());
        let dest_ids: std::collections::HashSet<String> = dest_store
            .load_all()
            .unwrap_or_default()
            .iter()
            .map(|t| t.frontmatter.id.clone())
            .collect();

        let mut conflicts = 0usize;
        let mut to_migrate = Vec::new();
        for task in &selected {
            if dest_ids.contains(&task.frontmatter.id) {
                if force {
                    to_migrate.push(*task);
                } else {
                    conflicts += 1;
                }
            } else {
                to_migrate.push(*task);
            }
        }

        if to_migrate.is_empty() {
            return format!(
                "No tasks to migrate. {} skipped due to ID conflicts (use force=true to overwrite).",
                conflicts
            );
        }

        if dry_run {
            let task_lines: Vec<String> = to_migrate
                .iter()
                .map(|t| {
                    let prefix = &t.frontmatter.id[..t.frontmatter.id.len().min(10)];
                    format!(
                        "  {} {} [{}]",
                        prefix, t.frontmatter.title, t.frontmatter.status
                    )
                })
                .collect();
            return format!(
                "Dry run: would {op_str} {} task(s) from {} to {}\n{}\n{}",
                to_migrate.len(),
                source_dir.display(),
                dest_dir.display(),
                task_lines.join("\n"),
                if conflicts > 0 {
                    format!("  ({conflicts} skipped due to conflicts)")
                } else {
                    String::new()
                }
            );
        }

        // Execute migration, re-assigning short_ids from destination's counter
        let dest_manifest = ManifestStore::with_path(dest_dir.join("manifest.json"));
        for task in &to_migrate {
            let mut task = (*task).clone();
            // Re-assign short_id from destination workspace counter
            if let Ok(sid) = dest_manifest.allocate_short_id() {
                task.frontmatter.short_id = Some(sid);
            }
            if let Err(e) = dest_store.save(&task) {
                return err_to_string(format!("Failed to save task: {e}"));
            }
            if let Some(ref stack) = task.frontmatter.stack {
                let _ = dest_manifest.register_stack(stack);
            }
            if !task.frontmatter.tags.is_empty() {
                let _ = dest_manifest.register_tags(&task.frontmatter.tags);
            }
        }

        if is_move {
            let source_manifest = ManifestStore::with_path(source_dir.join("manifest.json"));
            for task in &to_migrate {
                let _ = source_store.delete(&task.frontmatter.id);
            }
            if let Ok(remaining) = source_store.load_all() {
                let _ = source_manifest.prune_stacks_and_tags(&remaining);
            }
        }

        let mut result = format!(
            "{op_past} {} task(s) from {} to {}.",
            to_migrate.len(),
            source_dir.display(),
            dest_dir.display()
        );
        if conflicts > 0 {
            result.push_str(&format!(" Skipped {conflicts} due to conflicts."));
        }
        result
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
                    *info
                        .by_status
                        .entry(task.frontmatter.status.clone())
                        .or_default() += 1;
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
    by_stage: BTreeMap<String, usize>,
    by_stack: BTreeMap<String, StackStatsJson>,
    tags: BTreeMap<String, usize>,
}
