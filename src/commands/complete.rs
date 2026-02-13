use crate::cli::args::CompleteArgs;
use crate::error::Result;
use crate::model::task::TaskStatus;
use crate::storage::task_store::TaskStore;
use chrono::Utc;

pub fn execute(args: &CompleteArgs) -> Result<()> {
    let store = TaskStore::new();

    // Load, resolve prefix if needed
    let mut task = crate::commands::show::resolve_task_pub(&store, &args.id)?;

    task.frontmatter.status = TaskStatus::Done;
    task.frontmatter.modified = Utc::now();

    store.save(&task)?;
    println!("Completed: {} — {}", &task.frontmatter.id[..10], task.frontmatter.title);

    Ok(())
}
