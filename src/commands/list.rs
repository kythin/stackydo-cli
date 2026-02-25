use crate::cli::args::ListArgs;
use crate::commands::util::{
    active_stack_filter, format_task_row, parse_due_date, print_json, print_json_array,
    stack_filter_matches,
};
use crate::error::{Result, TodoError};
use crate::model::task::{Priority, TaskJson, TaskStatus};
use crate::storage::task_store::TaskStore;
use chrono::{Datelike, Utc};
use std::collections::BTreeMap;

pub fn execute(args: &ListArgs) -> Result<()> {
    let store = TaskStore::new();
    let mut tasks = store.load_all()?;

    // Apply stack_filter from stackydo.json (before CLI flags)
    if let Some(ref pattern) = active_stack_filter() {
        tasks.retain(|t| stack_filter_matches(pattern, t.frontmatter.stack.as_deref()));
    }

    // Hide soft-deleted tasks unless explicitly requested
    if args.status.as_deref() != Some("deleted") {
        tasks.retain(|t| t.frontmatter.status != TaskStatus::Deleted);
    }

    // Filter by status
    if let Some(ref status_str) = args.status {
        let s = status_str
            .parse::<TaskStatus>()
            .map_err(TodoError::Other)?;
        tasks.retain(|t| t.frontmatter.status == s);
    }

    // Filter by tag
    if let Some(ref tag) = args.tag {
        let tag_lower = tag.to_lowercase();
        tasks.retain(|t| {
            t.frontmatter
                .tags
                .iter()
                .any(|tt| tt.to_lowercase() == tag_lower)
        });
    }

    // Filter by priority
    if let Some(ref pri_str) = args.priority {
        let pri = pri_str
            .parse::<Priority>()
            .map_err(TodoError::Other)?;
        tasks.retain(|t| t.frontmatter.priority.as_ref() == Some(&pri));
    }

    // Filter by stack
    if let Some(ref stack) = args.stack {
        let stack_lower = stack.to_lowercase();
        tasks.retain(|t| {
            t.frontmatter
                .stack
                .as_ref()
                .map(|s| s.to_lowercase() == stack_lower)
                .unwrap_or(false)
        });
    }

    // Filter: overdue (due < now, not done/cancelled/deleted)
    if args.overdue {
        let now = Utc::now();
        tasks.retain(|t| {
            if let Some(due) = t.frontmatter.due {
                due < now
                    && t.frontmatter.status != TaskStatus::Done
                    && t.frontmatter.status != TaskStatus::Cancelled
                    && t.frontmatter.status != TaskStatus::Deleted
            } else {
                false
            }
        });
    }

    // Filter: --due-before
    if let Some(ref date_str) = args.due_before {
        let cutoff = parse_due_date(date_str)?;
        tasks.retain(|t| t.frontmatter.due.map(|d| d < cutoff).unwrap_or(false));
    }

    // Filter: --due-after
    if let Some(ref date_str) = args.due_after {
        let cutoff = parse_due_date(date_str)?;
        tasks.retain(|t| t.frontmatter.due.map(|d| d > cutoff).unwrap_or(false));
    }

    // Filter: --due-this-week (Monday–Sunday of current week)
    if args.due_this_week {
        let today = Utc::now().date_naive();
        let weekday = today.weekday().num_days_from_monday(); // 0=Mon
        let monday = today - chrono::Duration::days(weekday as i64);
        let sunday = monday + chrono::Duration::days(6);
        tasks.retain(|t| {
            if let Some(due) = t.frontmatter.due {
                let due_date = due.date_naive();
                due_date >= monday && due_date <= sunday
            } else {
                false
            }
        });
    }

    // Sort
    match args.sort.as_str() {
        "due" => tasks.sort_by(|a, b| a.frontmatter.due.cmp(&b.frontmatter.due)),
        "modified" => tasks.sort_by(|a, b| b.frontmatter.modified.cmp(&a.frontmatter.modified)),
        "priority" => tasks.sort_by(|a, b| a.frontmatter.priority.cmp(&b.frontmatter.priority)),
        _ => tasks.sort_by(|a, b| b.frontmatter.created.cmp(&a.frontmatter.created)),
    }

    if args.reverse {
        tasks.reverse();
    }

    // Limit
    if let Some(limit) = args.limit {
        tasks.truncate(limit);
    }

    // Group-by handling
    if let Some(ref group_field) = args.group_by {
        match group_field.as_str() {
            "stack" => {
                let mut groups: BTreeMap<String, Vec<_>> = BTreeMap::new();
                for task in &tasks {
                    let key = task
                        .frontmatter
                        .stack
                        .clone()
                        .unwrap_or_else(|| "(no stack)".to_string());
                    groups.entry(key).or_default().push(task);
                }

                if args.json {
                    let json_groups: BTreeMap<String, Vec<TaskJson>> = groups
                        .into_iter()
                        .map(|(k, v)| (k, v.into_iter().map(TaskJson::from).collect()))
                        .collect();
                    return print_json(&json_groups);
                }

                if groups.is_empty() {
                    println!("No tasks found.");
                    return Ok(());
                }

                for (stack_name, stack_tasks) in &groups {
                    println!(
                        "\n[{stack_name}] ({} task{})",
                        stack_tasks.len(),
                        if stack_tasks.len() == 1 { "" } else { "s" }
                    );
                    for task in stack_tasks {
                        println!("  {}", format_task_row(&task.frontmatter));
                    }
                }
                println!(
                    "\n({} task{} total)",
                    tasks.len(),
                    if tasks.len() == 1 { "" } else { "s" }
                );
                return Ok(());
            }
            other => {
                return Err(TodoError::Other(format!(
                    "Unknown group-by field: '{other}'. Supported: stack"
                )));
            }
        }
    }

    // JSON output
    if args.json {
        let json_tasks: Vec<TaskJson> = tasks.iter().map(TaskJson::from).collect();
        return print_json_array(&json_tasks);
    }

    // Human output
    if tasks.is_empty() {
        println!("No tasks found.");
        return Ok(());
    }

    for task in &tasks {
        println!("{}", format_task_row(&task.frontmatter));
    }

    println!(
        "\n({} task{})",
        tasks.len(),
        if tasks.len() == 1 { "" } else { "s" }
    );

    Ok(())
}
