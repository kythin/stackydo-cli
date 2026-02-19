use rmcp::{
    handler::server::wrapper::Parameters,
    model::{GetPromptResult, PromptMessage, PromptMessageRole},
    prompt_router, schemars,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::model::task::{TaskJson, TaskStatus};
use crate::storage::manifest_store::ManifestStore;
use crate::storage::task_store::TaskStore;
use chrono::Utc;
use std::collections::BTreeMap;

use super::StackydoMcp;

pub fn create_prompt_router() -> rmcp::handler::server::router::prompt::PromptRouter<StackydoMcp> {
    StackydoMcp::prompt_router()
}

// ── Prompt parameter structs ──

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct PlanWorkParams {
    /// The goal to break down into tasks
    pub goal: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct CaptureTodosParams {
    /// The text to extract actionable items from
    pub text: String,
}

// ── Helper functions ──

fn load_stats_summary() -> String {
    let store = TaskStore::new();
    let tasks = match store.load_all() {
        Ok(t) => t,
        Err(e) => return format!("Error loading tasks: {e}"),
    };
    let now = Utc::now();

    let total = tasks.len();
    let mut by_status: BTreeMap<String, usize> = BTreeMap::new();
    let mut overdue = 0usize;

    for task in &tasks {
        let status_str = task.frontmatter.status.to_string();
        *by_status.entry(status_str).or_default() += 1;

        if let Some(due) = task.frontmatter.due {
            if due < now
                && task.frontmatter.status != TaskStatus::Done
                && task.frontmatter.status != TaskStatus::Cancelled
            {
                overdue += 1;
            }
        }
    }

    let status_lines: Vec<String> = by_status
        .iter()
        .map(|(s, c)| format!("  {s}: {c}"))
        .collect();

    format!(
        "Total: {total}, Overdue: {overdue}\nBy status:\n{}",
        status_lines.join("\n")
    )
}

fn load_open_tasks_json() -> String {
    let store = TaskStore::new();
    let tasks = match store.load_all() {
        Ok(t) => t,
        Err(e) => return format!("Error: {e}"),
    };

    let open: Vec<TaskJson> = tasks
        .iter()
        .filter(|t| {
            t.frontmatter.status != TaskStatus::Done
                && t.frontmatter.status != TaskStatus::Cancelled
        })
        .map(TaskJson::from)
        .collect();

    serde_json::to_string_pretty(&open).unwrap_or_else(|e| format!("Error: {e}"))
}

fn load_stacks_and_tags_summary() -> String {
    let manifest_store = ManifestStore::new();
    let manifest = match manifest_store.load() {
        Ok(m) => m,
        Err(e) => return format!("Error: {e}"),
    };

    let stacks: Vec<&String> = manifest.stacks.iter().collect();
    let tags: Vec<&String> = manifest.tags.iter().collect();

    format!(
        "Known stacks: {}\nKnown tags: {}",
        if stacks.is_empty() {
            "(none)".to_string()
        } else {
            stacks.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
        },
        if tags.is_empty() {
            "(none)".to_string()
        } else {
            tags.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
        }
    )
}

fn load_recent_tasks_json() -> String {
    let store = TaskStore::new();
    let mut tasks = match store.load_all() {
        Ok(t) => t,
        Err(e) => return format!("Error: {e}"),
    };

    // Sort by most recently modified
    tasks.sort_by(|a, b| b.frontmatter.modified.cmp(&a.frontmatter.modified));
    tasks.truncate(20);

    let json_tasks: Vec<TaskJson> = tasks.iter().map(TaskJson::from).collect();
    serde_json::to_string_pretty(&json_tasks).unwrap_or_else(|e| format!("Error: {e}"))
}

// ── Prompt implementations ──

#[prompt_router]
impl StackydoMcp {
    #[rmcp::prompt(
        name = "triage",
        description = "Review all open tasks and suggest priority/status changes. Flags overdue items."
    )]
    async fn triage(&self) -> GetPromptResult {
        let stats = load_stats_summary();
        let open_tasks = load_open_tasks_json();

        GetPromptResult {
            description: Some("Triage open tasks".to_string()),
            messages: vec![
                PromptMessage::new_text(
                    PromptMessageRole::User,
                    format!(
                        "Review my open tasks and help me triage them. Suggest priority changes, \
                         status updates, and flag anything overdue or stale.\n\n\
                         ## Current Stats\n{stats}\n\n\
                         ## Open Tasks\n```json\n{open_tasks}\n```"
                    ),
                ),
            ],
        }
    }

    #[rmcp::prompt(
        name = "plan_work",
        description = "Break a goal into actionable stackydo tasks. Takes a goal description as input."
    )]
    async fn plan_work(&self, Parameters(params): Parameters<PlanWorkParams>) -> GetPromptResult {
        let stats = load_stats_summary();
        let stacks_tags = load_stacks_and_tags_summary();

        GetPromptResult {
            description: Some("Plan work for a goal".to_string()),
            messages: vec![
                PromptMessage::new_text(
                    PromptMessageRole::User,
                    format!(
                        "Break this goal into actionable tasks that I can create in stackydo. \
                         Use existing stacks/tags where appropriate, or suggest new ones.\n\n\
                         ## Goal\n{}\n\n\
                         ## Current Context\n{stats}\n\n{stacks_tags}\n\n\
                         For each task, specify: title, priority, stack, tags, and optionally a due date and body. \
                         Then use the create_task tool to create them.",
                        params.goal
                    ),
                ),
            ],
        }
    }

    #[rmcp::prompt(
        name = "daily_standup",
        description = "Summarize what's done, in progress, and blocked. Shows recently modified tasks."
    )]
    async fn daily_standup(&self) -> GetPromptResult {
        let stats = load_stats_summary();
        let recent = load_recent_tasks_json();

        GetPromptResult {
            description: Some("Daily standup summary".to_string()),
            messages: vec![
                PromptMessage::new_text(
                    PromptMessageRole::User,
                    format!(
                        "Give me a daily standup summary based on my tasks. Cover:\n\
                         - What's been completed recently\n\
                         - What's currently in progress\n\
                         - What's blocked and why\n\
                         - What should I focus on today\n\n\
                         ## Stats\n{stats}\n\n\
                         ## Recently Modified Tasks\n```json\n{recent}\n```"
                    ),
                ),
            ],
        }
    }

    #[rmcp::prompt(
        name = "capture_todos",
        description = "Extract actionable items from text and create tasks. Takes raw text as input."
    )]
    async fn capture_todos(
        &self,
        Parameters(params): Parameters<CaptureTodosParams>,
    ) -> GetPromptResult {
        let stacks_tags = load_stacks_and_tags_summary();

        GetPromptResult {
            description: Some("Capture TODOs from text".to_string()),
            messages: vec![
                PromptMessage::new_text(
                    PromptMessageRole::User,
                    format!(
                        "Extract actionable items from the following text and create stackydo tasks for each one. \
                         Use existing stacks and tags where they fit.\n\n\
                         ## Available Context\n{stacks_tags}\n\n\
                         ## Text to Process\n{}\n\n\
                         For each item, use the create_task tool with appropriate title, priority, stack, tags, and body.",
                        params.text
                    ),
                ),
            ],
        }
    }
}
