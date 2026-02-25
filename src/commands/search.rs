use crate::cli::args::SearchArgs;
use crate::commands::util::{
    active_stack_filter, format_task_row, print_json_array, stack_filter_matches,
};
use crate::error::Result;
use crate::model::task::{TaskJson, TaskStatus};
use crate::storage::task_store::TaskStore;

pub fn execute(args: &SearchArgs) -> Result<()> {
    let store = TaskStore::new();
    let mut results = store.search(&args.query)?;

    // Apply stack_filter from stackydo.json
    if let Some(ref pattern) = active_stack_filter() {
        results.retain(|t| stack_filter_matches(pattern, t.frontmatter.stack.as_deref()));
    }

    // Exclude soft-deleted tasks
    results.retain(|t| t.frontmatter.status != TaskStatus::Deleted);

    if args.json {
        let json_results: Vec<TaskJson> = results.iter().map(TaskJson::from).collect();
        return print_json_array(&json_results);
    }

    if results.is_empty() {
        println!("No tasks matching '{}'.", args.query);
        return Ok(());
    }

    for task in &results {
        println!("{}", format_task_row(&task.frontmatter));
    }
    println!(
        "\n({} result{})",
        results.len(),
        if results.len() == 1 { "" } else { "s" }
    );

    Ok(())
}
