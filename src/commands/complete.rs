use crate::cli::args::CompleteArgs;
use crate::commands::util::matches_filters;
use crate::error::{Result, TodoError};
use crate::model::task::TaskStatus;
use crate::storage::task_store::TaskStore;
use chrono::Utc;

pub fn execute(args: &CompleteArgs) -> Result<()> {
    let store = TaskStore::new();

    // Single-task mode
    if let Some(ref id) = args.id {
        let mut task = crate::commands::show::resolve_task_pub(&store, id)?;
        task.frontmatter.status = TaskStatus::Done;
        task.frontmatter.modified = Utc::now();
        store.save(&task)?;
        println!(
            "Completed: {} — {}",
            &task.frontmatter.id[..10],
            task.frontmatter.title
        );
        return Ok(());
    }

    // Bulk mode: requires --all
    if !args.all {
        return Err(TodoError::Other(
            "Bulk complete requires --all flag. Use filters (--status, --tag, --stack) to narrow scope."
                .into(),
        ));
    }

    // At least one filter should be specified for safety
    if args.status.is_none() && args.tag.is_none() && args.stack.is_none() {
        return Err(TodoError::Other(
            "Bulk complete requires at least one filter (--status, --tag, or --stack).".into(),
        ));
    }

    let status_filter = args
        .status
        .as_ref()
        .and_then(|s| s.parse::<TaskStatus>().ok());
    let tag_filter = args.tag.as_deref();
    let stack_filter = args.stack.as_deref();

    let tasks = store.load_all()?;
    let mut count = 0;

    for mut task in tasks {
        // Skip already-done tasks
        if task.frontmatter.status == TaskStatus::Done {
            continue;
        }

        if matches_filters(&task, status_filter.as_ref(), tag_filter, stack_filter) {
            task.frontmatter.status = TaskStatus::Done;
            task.frontmatter.modified = Utc::now();
            store.save(&task)?;
            println!(
                "  Completed: {} — {}",
                &task.frontmatter.id[..10],
                task.frontmatter.title
            );
            count += 1;
        }
    }

    println!(
        "\nCompleted {} task{}.",
        count,
        if count == 1 { "" } else { "s" }
    );

    Ok(())
}
