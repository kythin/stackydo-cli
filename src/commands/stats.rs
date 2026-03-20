use crate::cli::args::StatsArgs;
use crate::commands::util::{
    active_stack_filter, active_workflow, print_json, stack_filter_matches,
};
use crate::error::Result;
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
    by_stage: BTreeMap<String, usize>,
    by_stack: BTreeMap<String, StackStats>,
    tags: BTreeMap<String, usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    comments: Option<CommentStats>,
}

#[derive(Debug, Serialize)]
struct CommentStats {
    total_comments: usize,
    tasks_with_comments: usize,
    most_commented_task: Option<String>,
}

pub fn execute(args: &StatsArgs) -> Result<()> {
    let store = TaskStore::new();
    let mut tasks = store.load_all()?;
    let workflow = active_workflow();

    // Apply stack_filter from stackydo.json
    if let Some(ref pattern) = active_stack_filter() {
        tasks.retain(|t| stack_filter_matches(pattern, t.frontmatter.stack.as_deref()));
    }

    let now = Utc::now();

    let total = tasks.len();
    let mut by_status: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_stage: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_stack: BTreeMap<String, StackStats> = BTreeMap::new();
    let mut tags: BTreeMap<String, usize> = BTreeMap::new();
    let mut overdue = 0usize;

    let mut total_comments = 0usize;
    let mut tasks_with_comments = 0usize;
    let mut most_commented: Option<(&str, &str, usize)> = None; // (display_id, title, count)

    for task in &tasks {
        let status_str = task.frontmatter.status.clone();
        *by_status.entry(status_str.clone()).or_default() += 1;

        let stage = workflow.stage_for(&status_str);
        *by_stage.entry(stage.to_string()).or_default() += 1;

        // Overdue check
        if let Some(due) = task.frontmatter.due {
            if due < now && !workflow.is_terminal(&status_str) {
                overdue += 1;
            }
        }

        // Stack stats
        let stack_name = task
            .frontmatter
            .stack
            .clone()
            .unwrap_or_else(|| "(no stack)".to_string());
        let stack_entry = by_stack.entry(stack_name).or_insert_with(|| StackStats {
            total: 0,
            by_status: BTreeMap::new(),
        });
        stack_entry.total += 1;
        *stack_entry.by_status.entry(status_str).or_default() += 1;

        // Tags
        for tag in &task.frontmatter.tags {
            *tags.entry(tag.clone()).or_default() += 1;
        }

        // Comments
        let comment_count = task.frontmatter.comments.len();
        if comment_count > 0 {
            total_comments += comment_count;
            tasks_with_comments += 1;
            if most_commented.map_or(true, |(_, _, c)| comment_count > c) {
                most_commented = Some((
                    crate::commands::util::display_id(&task.frontmatter),
                    &task.frontmatter.title,
                    comment_count,
                ));
            }
        }
    }

    if args.json {
        let comment_stats = if total_comments > 0 {
            Some(CommentStats {
                total_comments,
                tasks_with_comments,
                most_commented_task: most_commented
                    .map(|(did, title, count)| format!("{did} — {title} ({count})")),
            })
        } else {
            None
        };
        let output = StatsOutput {
            total,
            overdue,
            by_status,
            by_stage,
            by_stack,
            tags,
            comments: comment_stats,
        };
        return print_json(&output);
    }

    // Human output
    println!("Total tasks: {total}");
    if overdue > 0 {
        println!("Overdue: {overdue}");
    }

    println!("\nBy stage:");
    for (stage, count) in &by_stage {
        println!("  {stage}: {count}");
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

    if total_comments > 0 {
        println!("\nComments:");
        println!("  Total comments: {total_comments}");
        println!("  Tasks with comments: {tasks_with_comments}");
        if let Some((did, title, count)) = most_commented {
            println!("  Most commented: {did} — {title} ({count})");
        }
    }

    Ok(())
}
