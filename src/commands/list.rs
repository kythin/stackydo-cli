use crate::cli::args::ListArgs;
use crate::commands::util::format_task_row;
use crate::error::Result;
use crate::model::task::{Priority, TaskStatus};
use crate::storage::task_store::TaskStore;

pub fn execute(args: &ListArgs) -> Result<()> {
    let store = TaskStore::new();
    let mut tasks = store.load_all()?;

    // Filter by status
    if let Some(ref status_str) = args.status {
        if let Ok(s) = status_str.parse::<TaskStatus>() {
            tasks.retain(|t| t.frontmatter.status == s);
        }
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
        if let Ok(pri) = pri_str.parse::<Priority>() {
            tasks.retain(|t| t.frontmatter.priority.as_ref() == Some(&pri));
        }
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

    // Print
    if tasks.is_empty() {
        println!("No tasks found.");
        return Ok(());
    }

    for task in &tasks {
        println!("{}", format_task_row(&task.frontmatter));
    }

    println!("\n({} task{})", tasks.len(), if tasks.len() == 1 { "" } else { "s" });

    Ok(())
}
