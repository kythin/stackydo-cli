use crate::cli::args::CompleteArgs;
use crate::commands::util::{active_workflow, display_id, matches_filters};
use crate::error::{Result, TodoError};
use crate::storage::task_store::TaskStore;
use chrono::Utc;

pub fn execute(args: &CompleteArgs) -> Result<()> {
    let store = TaskStore::new();

    // Single-task mode
    if let Some(ref id) = args.id {
        let mut task = crate::commands::show::resolve_task_pub(&store, id)?;
        task.frontmatter.status = "done".to_string();
        task.frontmatter.modified = Utc::now();
        store.save(&task)?;
        println!(
            "Completed: {} — {}",
            display_id(&task.frontmatter),
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

    let workflow = active_workflow();
    let status_filter = match &args.status {
        Some(s) => Some(workflow.validate_status(s).map_err(TodoError::Other)?),
        None => None,
    };
    let tag_filter = args.tag.as_deref();
    let stack_filter = args.stack.as_deref();

    let tasks = store.load_all()?;
    let mut count = 0;

    for mut task in tasks {
        // Skip tasks already in a terminal (archive) stage
        if workflow.is_terminal(&task.frontmatter.status) {
            continue;
        }

        if matches_filters(&task, status_filter.as_deref(), tag_filter, stack_filter) {
            task.frontmatter.status = "done".to_string();
            task.frontmatter.modified = Utc::now();
            store.save(&task)?;
            println!(
                "  Completed: {} — {}",
                display_id(&task.frontmatter),
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
