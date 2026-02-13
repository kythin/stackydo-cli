use crate::cli::args::DeleteArgs;
use crate::commands::util::matches_filters;
use crate::error::{Result, TodoError};
use crate::model::task::TaskStatus;
use crate::storage::task_store::TaskStore;

pub fn execute(args: &DeleteArgs) -> Result<()> {
    let store = TaskStore::new();

    // Single-task mode
    if let Some(ref id) = args.id {
        let task = crate::commands::show::resolve_task_pub(&store, id)?;

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

        store.delete(&task.frontmatter.id)?;
        println!("Deleted: {}", &task.frontmatter.id[..10]);
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

    let status_filter = args
        .status
        .as_ref()
        .and_then(|s| s.parse::<TaskStatus>().ok());
    let tag_filter = args.tag.as_deref();
    let stack_filter = args.stack.as_deref();

    let tasks = store.load_all()?;
    let mut count = 0;

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

    Ok(())
}
