use crate::cli::args::StatsArgs;
use crate::commands::util::{active_stack_filter, print_json, stack_filter_matches};
use crate::error::Result;
use crate::model::task::TaskStatus;
use crate::storage::task_store::TaskStore;
use chrono::Utc;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Serialize)]
struct StackStats {
    total: usize,
    by_status: BTreeMap<String, usize>,
}

#[derive(Debug, Serialize)]
struct StatsOutput {
    total: usize,
    overdue: usize,
    by_status: BTreeMap<String, usize>,
    by_stack: BTreeMap<String, StackStats>,
    tags: BTreeMap<String, usize>,
}

pub fn execute(args: &StatsArgs) -> Result<()> {
    let store = TaskStore::new();
    let mut tasks = store.load_all()?;

    // Apply stack_filter from stackydo.json
    if let Some(ref pattern) = active_stack_filter() {
        tasks.retain(|t| stack_filter_matches(pattern, t.frontmatter.stack.as_deref()));
    }

    // Exclude soft-deleted tasks from stats
    tasks.retain(|t| t.frontmatter.status != TaskStatus::Deleted);

    let now = Utc::now();

    let total = tasks.len();
    let mut by_status: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_stack: BTreeMap<String, StackStats> = BTreeMap::new();
    let mut tags: BTreeMap<String, usize> = BTreeMap::new();
    let mut overdue = 0usize;

    for task in &tasks {
        let status_str = task.frontmatter.status.to_string();
        *by_status.entry(status_str.clone()).or_default() += 1;

        // Overdue check
        if let Some(due) = task.frontmatter.due {
            if due < now
                && task.frontmatter.status != TaskStatus::Done
                && task.frontmatter.status != TaskStatus::Cancelled
            {
                overdue += 1;
            }
        }

        // Stack stats
        let stack_name = task
            .frontmatter
            .stack
            .clone()
            .unwrap_or_else(|| "(no stack)".to_string());
        let stack_entry = by_stack
            .entry(stack_name)
            .or_insert_with(|| StackStats {
                total: 0,
                by_status: BTreeMap::new(),
            });
        stack_entry.total += 1;
        *stack_entry
            .by_status
            .entry(status_str)
            .or_default() += 1;

        // Tags
        for tag in &task.frontmatter.tags {
            *tags.entry(tag.clone()).or_default() += 1;
        }
    }

    if args.json {
        let output = StatsOutput {
            total,
            overdue,
            by_status,
            by_stack,
            tags,
        };
        return print_json(&output);
    }

    // Human output
    println!("Total tasks: {total}");
    if overdue > 0 {
        println!("Overdue: {overdue}");
    }

    println!("\nBy status:");
    for (status, count) in &by_status {
        println!("  {status}: {count}");
    }

    println!("\nBy stack:");
    for (stack, stats) in &by_stack {
        let breakdown: Vec<String> = stats
            .by_status
            .iter()
            .map(|(s, c)| format!("{s}:{c}"))
            .collect();
        println!(
            "  {stack}: {} total ({})",
            stats.total,
            breakdown.join(", ")
        );
    }

    if !tags.is_empty() {
        println!("\nTop tags:");
        let mut tag_vec: Vec<_> = tags.iter().collect();
        tag_vec.sort_by(|a, b| b.1.cmp(a.1));
        for (tag, count) in tag_vec {
            println!("  {tag}: {count}");
        }
    }

    Ok(())
}
