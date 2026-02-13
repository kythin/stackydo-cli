use crate::cli::args::SearchArgs;
use crate::commands::util::format_task_row;
use crate::error::Result;
use crate::storage::task_store::TaskStore;

pub fn execute(args: &SearchArgs) -> Result<()> {
    let store = TaskStore::new();
    let results = store.search(&args.query)?;

    if results.is_empty() {
        println!("No tasks matching '{}'.", args.query);
        return Ok(());
    }

    for task in &results {
        println!("{}", format_task_row(&task.frontmatter));
    }
    println!("\n({} result{})", results.len(), if results.len() == 1 { "" } else { "s" });

    Ok(())
}
