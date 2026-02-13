use crate::cli::args::ListArgs;
use crate::error::Result;
use crate::model::task::{Priority, TaskStatus};
use crate::storage::task_store::TaskStore;

pub fn execute(args: &ListArgs) -> Result<()> {
    let store = TaskStore::new();
    let mut tasks = store.load_all()?;

    // Filter by status
    if let Some(ref status_str) = args.status {
        let status = parse_status(status_str);
        if let Some(s) = status {
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
        let fm = &task.frontmatter;
        let pri = fm
            .priority
            .as_ref()
            .map(|p| format!("[{p}]"))
            .unwrap_or_default();
        let due = fm
            .due
            .map(|d| format!(" due:{}", d.format("%Y-%m-%d")))
            .unwrap_or_default();
        let tags = if fm.tags.is_empty() {
            String::new()
        } else {
            format!(" #{}", fm.tags.join(" #"))
        };

        println!(
            "{status:<12} {id:.10}  {pri:<10} {title}{due}{tags}",
            status = fm.status,
            id = fm.id,
            pri = pri,
            title = fm.title,
        );
    }

    println!("\n({} task{})", tasks.len(), if tasks.len() == 1 { "" } else { "s" });

    Ok(())
}

fn parse_status(s: &str) -> Option<TaskStatus> {
    match s.to_lowercase().as_str() {
        "todo" => Some(TaskStatus::Todo),
        "in_progress" | "inprogress" | "doing" => Some(TaskStatus::InProgress),
        "done" => Some(TaskStatus::Done),
        "blocked" => Some(TaskStatus::Blocked),
        "cancelled" | "canceled" => Some(TaskStatus::Cancelled),
        _ => None,
    }
}
