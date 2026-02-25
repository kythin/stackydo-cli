use crate::cli::args::DeleteArgs;
use crate::commands::util::matches_filters;
use crate::error::{Result, TodoError};
use crate::model::task::TaskStatus;
use crate::storage::manifest_store::ManifestStore;
use crate::storage::task_store::TaskStore;

pub fn execute(args: &DeleteArgs) -> Result<()> {
    let store = TaskStore::new();
    let manifest_store = ManifestStore::new();
    let soft_delete = manifest_store
        .load()
        .map(|m| m.settings.soft_delete)
        .unwrap_or(false);

    // Single-task mode
    if let Some(ref id) = args.id {
        let mut task = crate::commands::show::resolve_task_pub(&store, id)?;

        // Warn about orphaned subtasks
        if !task.frontmatter.subtask_ids.is_empty() {
            eprintln!(
                "Warning: this task has {} subtask(s) that will become orphaned.",
                task.frontmatter.subtask_ids.len()
            );
        }

        // Warn about tasks that depend on this one
        if !task.frontmatter.dependencies.is_empty() {
            eprintln!(
                "Warning: this task has {} dependency link(s).",
                task.frontmatter.dependencies.len()
            );
        }

        if !args.force {
            eprint!(
                "Delete '{}' ({})? [y/N] ",
                task.frontmatter.title,
                &task.frontmatter.id[..10]
            );
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if !input.trim().eq_ignore_ascii_case("y") {
                println!("Cancelled.");
                return Ok(());
            }
        }

        if soft_delete {
            task.frontmatter.status = TaskStatus::Deleted;
            task.frontmatter.modified = chrono::Utc::now();
            store.save(&task)?;
            println!("Soft-deleted: {}", &task.frontmatter.id[..10]);
        } else {
            // Clear parent's subtask reference if this task has a parent
            if let Some(ref parent_id) = task.frontmatter.parent_id {
                if let Ok(mut parent) = store.load(parent_id) {
                    parent.frontmatter.subtask_ids.retain(|s| s != &task.frontmatter.id);
                    parent.frontmatter.modified = chrono::Utc::now();
                    let _ = store.save(&parent);
                }
            }

            store.delete(&task.frontmatter.id)?;
            println!("Deleted: {}", &task.frontmatter.id[..10]);
            let remaining = store.load_all()?;
            manifest_store.prune_stacks_and_tags(&remaining)?;
        }
        return Ok(());
    }

    // Bulk mode: requires --force --all
    if !args.all {
        return Err(TodoError::Other(
            "Bulk delete requires --all flag. Use filters (--status, --tag, --stack) to narrow scope."
                .into(),
        ));
    }
    if !args.force {
        return Err(TodoError::Other(
            "Bulk delete requires --force flag for safety.".into(),
        ));
    }

    // At least one filter should be specified for safety
    if args.status.is_none() && args.tag.is_none() && args.stack.is_none() {
        return Err(TodoError::Other(
            "Bulk delete requires at least one filter (--status, --tag, or --stack).".into(),
        ));
    }

    let status_filter = match &args.status {
        Some(s) => Some(s.parse::<TaskStatus>().map_err(TodoError::Other)?),
        None => None,
    };
    let tag_filter = args.tag.as_deref();
    let stack_filter = args.stack.as_deref();

    let mut tasks = store.load_all()?;
    let mut count = 0;

    if soft_delete {
        for task in &mut tasks {
            if matches_filters(task, status_filter.as_ref(), tag_filter, stack_filter) {
                task.frontmatter.status = TaskStatus::Deleted;
                task.frontmatter.modified = chrono::Utc::now();
                store.save(task)?;
                println!(
                    "  Soft-deleted: {} — {}",
                    &task.frontmatter.id[..10],
                    task.frontmatter.title
                );
                count += 1;
            }
        }
        println!(
            "\nSoft-deleted {} task{}.",
            count,
            if count == 1 { "" } else { "s" }
        );
    } else {
        for task in &tasks {
            if matches_filters(task, status_filter.as_ref(), tag_filter, stack_filter) {
                store.delete(&task.frontmatter.id)?;
                println!(
                    "  Deleted: {} — {}",
                    &task.frontmatter.id[..10],
                    task.frontmatter.title
                );
                count += 1;
            }
        }

        println!(
            "\nDeleted {} task{}.",
            count,
            if count == 1 { "" } else { "s" }
        );

        if count > 0 {
            let remaining = store.load_all()?;
            manifest_store.prune_stacks_and_tags(&remaining)?;
        }
    }

    Ok(())
}
