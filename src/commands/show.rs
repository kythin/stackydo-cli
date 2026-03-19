use crate::cli::args::ShowArgs;
use crate::commands::util::print_json;
use crate::error::Result;
use crate::model::task::TaskJson;
use crate::storage::task_store::TaskStore;

pub fn execute(args: &ShowArgs) -> Result<()> {
    let store = TaskStore::new();
    let task = resolve_task_pub(&store, &args.id)?;

    if args.json {
        let json_task = TaskJson::from(&task);
        return print_json(&json_task);
    }

    let fm = &task.frontmatter;

    println!("ID:       {}", fm.id);
    if let Some(ref sid) = fm.short_id {
        println!("Short ID: {sid}");
    }
    println!("Title:    {}", fm.title);
    println!("Status:   {}", fm.status);
    if let Some(ref p) = fm.priority {
        println!("Priority: {p}");
    }
    if !fm.tags.is_empty() {
        println!("Tags:     {}", fm.tags.join(", "));
    }
    if let Some(ref stack) = fm.stack {
        println!("Stack:    {stack}");
    }
    if let Some(due) = fm.due {
        println!("Due:      {}", due.format("%Y-%m-%d %H:%M %Z"));
    }
    println!("Created:  {}", fm.created.format("%Y-%m-%d %H:%M %Z"));
    println!("Modified: {}", fm.modified.format("%Y-%m-%d %H:%M %Z"));

    // Context
    println!("\n--- Context ---");
    println!("Working dir: {}", fm.context.working_dir);
    if let Some(ref p) = fm.context.path {
        print!("Context path: {p}");
        if let Some(line) = fm.context.line {
            print!(":{line}");
            if let Some(col) = fm.context.column {
                print!(":{col}");
            }
        }
        println!();
    }
    if let Some(ref lf) = fm.context.lookfor {
        println!("Lookfor: {lf}");
    }
    if let Some(ref branch) = fm.context.git_branch {
        println!("Git branch: {branch}");
    }
    if let Some(ref remote) = fm.context.git_remote {
        println!("Git remote: {remote}");
    }
    if let Some(ref commit) = fm.context.git_commit {
        println!("Git commit: {commit}");
    }
    if let Some(ref prev) = fm.context.session_prev_task_id {
        println!("Prev task: {prev}");
    }

    // Parent / subtasks
    if let Some(ref parent_id) = fm.parent_id {
        println!("\nParent:   {parent_id}");
    }
    if !fm.subtask_ids.is_empty() {
        println!("\n--- Subtasks ---");
        for sid in &fm.subtask_ids {
            println!("  {sid}");
        }
    }

    // Dependencies
    if !fm.dependencies.is_empty() {
        println!("\n--- Dependencies ---");
        for dep in &fm.dependencies {
            println!("  {:?} -> {}", dep.dep_type, dep.task_id);
        }
    }

    // Body
    if !task.body.is_empty() {
        println!("\n--- Body ---");
        println!("{}", task.body);
    }

    Ok(())
}

use crate::model::task::Task;

/// Resolve a task ID by exact match, short ID, or unique ULID prefix.
/// Public so other commands (complete, delete) can reuse it.
pub fn resolve_task_pub(store: &TaskStore, id_or_prefix: &str) -> crate::error::Result<Task> {
    // Try exact ULID match first (file lookup, fast path)
    if let Ok(task) = store.load(id_or_prefix) {
        return Ok(task);
    }

    let all = store.load_all()?;
    let input_lower = id_or_prefix.to_lowercase();

    // Try exact short_id match (e.g. "SD42")
    let short_matches: Vec<_> = all
        .iter()
        .filter(|t| {
            t.frontmatter
                .short_id
                .as_ref()
                .is_some_and(|s| s.to_lowercase() == input_lower)
        })
        .collect();

    match short_matches.len() {
        1 => return Ok(short_matches[0].clone()),
        n if n > 1 => {
            return Err(crate::error::TodoError::Other(format!(
                "Duplicate short ID '{id_or_prefix}': matches {n} tasks. Use the full ULID instead."
            )))
        }
        _ => {} // 0 — fall through to prefix match
    }

    // Try ULID prefix match
    let prefix_matches: Vec<_> = all
        .into_iter()
        .filter(|t| t.frontmatter.id.to_lowercase().starts_with(&input_lower))
        .collect();

    match prefix_matches.len() {
        0 => Err(crate::error::TodoError::TaskNotFound(id_or_prefix.into())),
        1 => Ok(prefix_matches.into_iter().next().expect("len confirmed 1 item")),
        n => Err(crate::error::TodoError::Other(format!(
            "Ambiguous ID prefix '{id_or_prefix}': matches {n} tasks. Be more specific."
        ))),
    }
}
