use crate::cli::args::StacksArgs;
use crate::commands::util::print_json;
use crate::error::Result;
use crate::storage::manifest_store::ManifestStore;
use crate::storage::task_store::TaskStore;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Serialize)]
struct StackInfo {
    total: usize,
    by_status: BTreeMap<String, usize>,
}

pub fn execute(args: &StacksArgs) -> Result<()> {
    let store = TaskStore::new();
    let manifest_store = ManifestStore::new();
    let tasks = store.load_all()?;
    let manifest = manifest_store.load()?;

    // Collect all known stacks from tasks + manifest
    let mut all_stacks: BTreeSet<String> = manifest.stacks.iter().cloned().collect();
    for task in &tasks {
        if let Some(ref stack) = task.frontmatter.stack {
            all_stacks.insert(stack.clone());
        }
    }

    // Build per-stack stats
    let mut stack_infos: BTreeMap<String, StackInfo> = BTreeMap::new();
    for stack_name in &all_stacks {
        stack_infos.insert(
            stack_name.clone(),
            StackInfo {
                total: 0,
                by_status: BTreeMap::new(),
            },
        );
    }

    for task in &tasks {
        if let Some(ref stack) = task.frontmatter.stack {
            if let Some(info) = stack_infos.get_mut(stack) {
                info.total += 1;
                let status_str = task.frontmatter.status.to_string();
                *info.by_status.entry(status_str).or_default() += 1;
            }
        }
    }

    if args.json {
        return print_json(&stack_infos);
    }

    // Human output
    if stack_infos.is_empty() {
        println!("No stacks found.");
        return Ok(());
    }

    for (name, info) in &stack_infos {
        let breakdown: Vec<String> = info
            .by_status
            .iter()
            .map(|(s, c)| format!("{s}:{c}"))
            .collect();
        let breakdown_str = if breakdown.is_empty() {
            String::new()
        } else {
            format!(" ({})", breakdown.join(", "))
        };
        println!("{name}: {} total{breakdown_str}", info.total);
    }

    Ok(())
}
