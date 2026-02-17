use crate::cli::args::UpdateArgs;
use crate::commands::util::parse_due_date;
use crate::error::{Result, TodoError};
use crate::model::task::{Dependency, DependencyType, Priority, TaskStatus};
use crate::storage::manifest_store::ManifestStore;
use crate::storage::task_store::TaskStore;
use chrono::Utc;

pub fn execute(args: &UpdateArgs) -> Result<()> {
    let store = TaskStore::new();
    let manifest_store = ManifestStore::new();
    let mut task = crate::commands::show::resolve_task_pub(&store, &args.id)?;
    let task_id = task.frontmatter.id.clone();

    let mut changed = false;

    // Title
    if let Some(ref title) = args.title {
        task.frontmatter.title = title.clone();
        changed = true;
    }

    // Status
    if let Some(ref status_str) = args.status {
        let status = status_str
            .parse::<TaskStatus>()
            .map_err(TodoError::Other)?;
        task.frontmatter.status = status;
        changed = true;
    }

    // Priority ("none" clears)
    if let Some(ref pri_str) = args.priority {
        if pri_str.eq_ignore_ascii_case("none") || pri_str.is_empty() {
            task.frontmatter.priority = None;
        } else {
            task.frontmatter.priority = Some(
                pri_str
                    .parse::<Priority>()
                    .map_err(TodoError::Other)?,
            );
        }
        changed = true;
    }

    // Tags (replaces; empty string clears)
    if let Some(ref tags_csv) = args.tags {
        if tags_csv.is_empty() {
            task.frontmatter.tags = Vec::new();
        } else {
            let tags: Vec<String> = tags_csv
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if let Err(e) = manifest_store.register_tags(&tags) {
                eprintln!("Warning: failed to register tags in manifest: {e}");
            }
            task.frontmatter.tags = tags;
        }
        changed = true;
    }

    // Stack (empty string clears)
    if let Some(ref stack) = args.stack {
        let stack = stack.trim().to_string();
        if stack.is_empty() {
            task.frontmatter.stack = None;
        } else {
            if let Err(e) = manifest_store.register_stack(&stack) {
                eprintln!("Warning: failed to register stack in manifest: {e}");
            }
            task.frontmatter.stack = Some(stack);
        }
        changed = true;
    }

    // Due (empty string clears)
    if let Some(ref due_str) = args.due {
        if due_str.is_empty() {
            task.frontmatter.due = None;
        } else {
            task.frontmatter.due = Some(parse_due_date(due_str)?);
        }
        changed = true;
    }

    // Clear dependencies if requested
    if args.clear_deps && !task.frontmatter.dependencies.is_empty() {
        task.frontmatter.dependencies.clear();
        changed = true;
    }

    // Dependencies: blocked-by
    for raw_id in &args.blocked_by {
        let dep_task = crate::commands::show::resolve_task_pub(&store, raw_id)?;
        task.frontmatter.dependencies.push(Dependency {
            task_id: dep_task.frontmatter.id,
            dep_type: DependencyType::BlockedBy,
        });
        changed = true;
    }

    // Dependencies: blocks
    for raw_id in &args.blocks {
        let dep_task = crate::commands::show::resolve_task_pub(&store, raw_id)?;
        task.frontmatter.dependencies.push(Dependency {
            task_id: dep_task.frontmatter.id,
            dep_type: DependencyType::Blocks,
        });
        changed = true;
    }

    // Dependencies: related-to
    for raw_id in &args.related_to {
        let dep_task = crate::commands::show::resolve_task_pub(&store, raw_id)?;
        task.frontmatter.dependencies.push(Dependency {
            task_id: dep_task.frontmatter.id,
            dep_type: DependencyType::RelatedTo,
        });
        changed = true;
    }

    // Parent wiring
    if let Some(ref parent_raw) = args.parent {
        let mut parent_task = crate::commands::show::resolve_task_pub(&store, parent_raw)?;
        let parent_id = parent_task.frontmatter.id.clone();
        task.frontmatter.parent_id = Some(parent_id);

        if !parent_task.frontmatter.subtask_ids.contains(&task_id) {
            parent_task.frontmatter.subtask_ids.push(task_id.clone());
            parent_task.frontmatter.modified = Utc::now();
            store.save(&parent_task)?;
        }
        changed = true;
    }

    // Body (append)
    if !args.body.is_empty() {
        let extra = args.body.join(" ");
        if task.body.is_empty() {
            task.body = extra;
        } else {
            task.body.push('\n');
            task.body.push_str(&extra);
        }
        changed = true;
    }

    // Note (timestamped append)
    if let Some(ref note_text) = args.note {
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
        println!("No changes specified.");
        return Ok(());
    }

    task.frontmatter.modified = Utc::now();
    store.save(&task)?;

    println!(
        "Updated: {} — {}",
        &task.frontmatter.id[..10],
        task.frontmatter.title
    );

    Ok(())
}
