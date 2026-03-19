use crate::cli::args::SearchArgs;
use crate::commands::util::{
    active_stack_filter, active_workflow, apply_filters, apply_pagination, apply_sort,
    effective_limit, format_task_row, print_json, print_json_array, print_pagination_footer,
    stack_filter_matches, FilterParams,
};
use crate::error::{Result, TodoError};
use crate::model::task::{TaskJson, TaskSummaryJson};
use crate::storage::task_store::TaskStore;
use std::collections::BTreeMap;

pub fn execute(args: &SearchArgs) -> Result<()> {
    let store = TaskStore::new();
    let mut results = store.search(&args.query)?;

    // Apply stack_filter from stackydo.json
    if let Some(ref pattern) = active_stack_filter() {
        results.retain(|t| stack_filter_matches(pattern, t.frontmatter.stack.as_deref()));
    }

    // Hide archive-stage tasks by default unless explicitly filtering by status or stage
    if args.status.is_none() && args.stage.is_none() {
        let workflow = active_workflow();
        results.retain(|t| !workflow.is_terminal(&t.frontmatter.status));
    }

    // Apply filters
    apply_filters(
        &mut results,
        &FilterParams {
            status: args.status.as_deref(),
            stage: args.stage.as_deref(),
            tag: args.tag.as_deref(),
            priority: args.priority.as_deref(),
            stack: args.stack.as_deref(),
            overdue: args.overdue,
            due_before: args.due_before.as_deref(),
            due_after: args.due_after.as_deref(),
            due_this_week: args.due_this_week,
        },
    )?;

    // Sort
    apply_sort(&mut results, &args.sort, args.reverse)?;

    // Group-by handling (no pagination — show all matching results grouped)
    if let Some(ref group_field) = args.group_by {
        match group_field.as_str() {
            "stack" => {
                let mut groups: BTreeMap<String, Vec<_>> = BTreeMap::new();
                for task in &results {
                    let key = task
                        .frontmatter
                        .stack
                        .clone()
                        .unwrap_or_else(|| "(no stack)".to_string());
                    groups.entry(key).or_default().push(task);
                }

                if args.json {
                    if args.full {
                        let json_groups: BTreeMap<String, Vec<TaskJson>> = groups
                            .into_iter()
                            .map(|(k, v)| (k, v.into_iter().map(TaskJson::from).collect()))
                            .collect();
                        return print_json(&json_groups);
                    } else {
                        let json_groups: BTreeMap<String, Vec<TaskSummaryJson>> = groups
                            .into_iter()
                            .map(|(k, v)| (k, v.into_iter().map(TaskSummaryJson::from).collect()))
                            .collect();
                        return print_json(&json_groups);
                    }
                }

                if groups.is_empty() {
                    println!("No tasks matching '{}'.", args.query);
                    return Ok(());
                }

                for (stack_name, stack_tasks) in &groups {
                    println!(
                        "\n[{stack_name}] ({} result{})",
                        stack_tasks.len(),
                        if stack_tasks.len() == 1 { "" } else { "s" }
                    );
                    for task in stack_tasks {
                        println!("  {}", format_task_row(&task.frontmatter));
                    }
                }
                return Ok(());
            }
            other => {
                return Err(TodoError::Other(format!(
                    "Unknown group-by field: '{other}'. Supported: stack"
                )));
            }
        }
    }

    // Pagination (only for non-grouped output)
    let limit = effective_limit(args.limit);
    let info = apply_pagination(&mut results, args.offset, limit);

    // JSON output
    if args.json {
        if args.full {
            let json_results: Vec<TaskJson> = results.iter().map(TaskJson::from).collect();
            return print_json_array(&json_results);
        } else {
            let json_results: Vec<TaskSummaryJson> =
                results.iter().map(TaskSummaryJson::from).collect();
            return print_json_array(&json_results);
        }
    }

    if results.is_empty() {
        println!("No tasks matching '{}'.", args.query);
        return Ok(());
    }

    for task in &results {
        println!("{}", format_task_row(&task.frontmatter));
    }
    print_pagination_footer(&info, "result");

    Ok(())
}
