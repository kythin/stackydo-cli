use crate::cli::args::MigrateArgs;
use crate::error::{Result, TodoError};
use crate::storage::manifest_store::ManifestStore;
use crate::storage::task_store::TaskStore;
use crate::storage::workspace::{
    discover_workspaces, find_git_repo_root, resolve_workspace_path, WorkspaceInfo,
};
use std::collections::HashSet;
use std::path::Path;

pub fn execute(args: &MigrateArgs) -> Result<()> {
    let interactive = args.source.is_none() && args.dest.is_none();

    if interactive {
        execute_interactive(args)
    } else {
        execute_non_interactive(args)
    }
}

// ── Non-interactive mode ──────────────────────────────────────────────────

fn execute_non_interactive(args: &MigrateArgs) -> Result<()> {
    let source_path = args
        .source
        .as_deref()
        .ok_or_else(|| TodoError::Other("--source is required in non-interactive mode".into()))?;
    let dest_path = args
        .dest
        .as_deref()
        .ok_or_else(|| TodoError::Other("--dest is required in non-interactive mode".into()))?;

    if !args.r#move && !args.copy {
        return Err(TodoError::Other(
            "Specify --move or --copy in non-interactive mode".into(),
        ));
    }

    if args.task.is_empty() && args.stack.is_empty() && !args.all {
        return Err(TodoError::Other(
            "Specify --task, --stack, or --all to select tasks".into(),
        ));
    }

    let source_dir = resolve_workspace_path(source_path)
        .map_err(|e| TodoError::Other(format!("Invalid source: {e}")))?;
    let dest_dir = resolve_workspace_path(dest_path)
        .map_err(|e| TodoError::Other(format!("Invalid dest: {e}")))?;

    let source_store = TaskStore::with_root(source_dir.clone());
    let all_tasks = source_store.load_all()?;

    // Select tasks
    let selected = select_tasks_by_args(&all_tasks, &args.task, &args.stack, args.all)?;

    if selected.is_empty() {
        println!("No tasks matched the given filters.");
        return Ok(());
    }

    let is_move = args.r#move;
    let git_commit = args.git_commit.unwrap_or(false);

    do_migrate(
        &source_dir,
        &dest_dir,
        &selected,
        is_move,
        args.dry_run,
        args.force,
        git_commit,
    )
}

// ── Interactive mode ──────────────────────────────────────────────────────

fn execute_interactive(args: &MigrateArgs) -> Result<()> {
    let workspaces = discover_workspaces();
    if workspaces.len() < 2 {
        return Err(TodoError::Other(
            "Need at least 2 workspaces to migrate between. \
             Use `stackydo init --here` to create a project workspace."
                .into(),
        ));
    }

    // 1. Select source
    let source_labels: Vec<String> = workspaces.iter().map(|w| w.label()).collect();
    let source_idx = dialoguer::Select::new()
        .with_prompt("Select source workspace")
        .items(&source_labels)
        .default(0)
        .interact()
        .map_err(|e| TodoError::Other(format!("Selection cancelled: {e}")))?;

    let source_ws = &workspaces[source_idx];

    // 2. Select destination
    let dest_options: Vec<(usize, &WorkspaceInfo)> = workspaces
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != source_idx)
        .collect();

    if dest_options.is_empty() {
        return Err(TodoError::Other(
            "No other workspaces available as destination.".into(),
        ));
    }

    let dest_labels: Vec<String> = dest_options.iter().map(|(_, w)| w.label()).collect();
    eprintln!(
        "  (Don't see your destination? Cancel and run `stackydo init --here` from the target project first.)"
    );
    let dest_idx = dialoguer::Select::new()
        .with_prompt("Select destination workspace")
        .items(&dest_labels)
        .default(0)
        .interact()
        .map_err(|e| TodoError::Other(format!("Selection cancelled: {e}")))?;

    let dest_ws = dest_options[dest_idx].1;

    // Load source tasks
    let source_store = TaskStore::with_root(source_ws.store_dir.clone());
    let all_tasks = source_store.load_all()?;

    if all_tasks.is_empty() {
        println!("Source workspace has no tasks.");
        return Ok(());
    }

    // 3. Select stacks
    let stack_names: Vec<String> = {
        let mut set = std::collections::BTreeSet::new();
        for t in &all_tasks {
            if let Some(ref s) = t.frontmatter.stack {
                set.insert(s.clone());
            }
        }
        set.into_iter().collect()
    };
    let has_unstacked = all_tasks
        .iter()
        .any(|t| t.frontmatter.stack.is_none());

    let mut stack_choices = vec!["(all)".to_string()];
    stack_choices.append(&mut stack_names.clone());
    if has_unstacked {
        stack_choices.push("(no stack)".to_string());
    }

    let stack_selections = dialoguer::MultiSelect::new()
        .with_prompt("Select stack(s) to migrate from")
        .items(&stack_choices)
        .interact()
        .map_err(|e| TodoError::Other(format!("Selection cancelled: {e}")))?;

    let select_all_stacks = stack_selections.contains(&0);
    let selected_stacks: HashSet<Option<String>> = if select_all_stacks {
        // All stacks + unstacked
        let mut set: HashSet<Option<String>> = stack_names
            .iter()
            .map(|s| Some(s.clone()))
            .collect();
        set.insert(None);
        set
    } else {
        let mut set = HashSet::new();
        for &idx in &stack_selections {
            if idx == 0 {
                continue; // "(all)" already handled
            }
            let choice = &stack_choices[idx];
            if choice == "(no stack)" {
                set.insert(None);
            } else {
                set.insert(Some(choice.clone()));
            }
        }
        set
    };

    if selected_stacks.is_empty() {
        println!("No stacks selected.");
        return Ok(());
    }

    // Filter tasks by selected stacks
    let stack_filtered: Vec<&crate::model::task::Task> = all_tasks
        .iter()
        .filter(|t| selected_stacks.contains(&t.frontmatter.stack))
        .collect();

    if stack_filtered.is_empty() {
        println!("No tasks in selected stacks.");
        return Ok(());
    }

    // 4. Select tasks
    let mut task_choices: Vec<String> = vec!["(all)".to_string()];
    for t in &stack_filtered {
        let prefix = &t.frontmatter.id[..t.frontmatter.id.len().min(10)];
        task_choices.push(format!(
            "{} {} [{}]",
            prefix, t.frontmatter.title, t.frontmatter.status
        ));
    }

    let task_selections = dialoguer::MultiSelect::new()
        .with_prompt("Select task(s) to migrate")
        .items(&task_choices)
        .interact()
        .map_err(|e| TodoError::Other(format!("Selection cancelled: {e}")))?;

    let select_all_tasks = task_selections.contains(&0);
    let selected_tasks: Vec<&crate::model::task::Task> = if select_all_tasks {
        stack_filtered
    } else {
        task_selections
            .iter()
            .filter(|&&idx| idx > 0)
            .filter_map(|&idx| stack_filtered.get(idx - 1).copied())
            .collect()
    };

    if selected_tasks.is_empty() {
        println!("No tasks selected.");
        return Ok(());
    }

    // 5. Git commit option
    let any_git = source_ws.git_repo_root.is_some() || dest_ws.git_repo_root.is_some();
    let git_commit = if any_git {
        dialoguer::Confirm::new()
            .with_prompt("Create git commits for easier rollback?")
            .default(true)
            .interact()
            .unwrap_or(true)
    } else {
        false
    };

    // 6. Summary
    println!();
    println!("Migration summary:");
    println!("  Source: {}", source_ws.label());
    println!("  Dest:   {}", dest_ws.label());
    println!("  Tasks:  {}", selected_tasks.len());
    if git_commit {
        println!("  Git commits: yes");
    }

    // 7. Operation
    let op_choices = ["Move to destination", "Copy to destination", "Cancel"];
    let op_idx = dialoguer::Select::new()
        .with_prompt("Operation")
        .items(&op_choices)
        .default(0)
        .interact()
        .map_err(|e| TodoError::Other(format!("Selection cancelled: {e}")))?;

    let is_move = match op_idx {
        0 => true,
        1 => false,
        _ => {
            println!("Cancelled.");
            return Ok(());
        }
    };

    // Collect task IDs for the migration
    let task_ids: Vec<String> = selected_tasks
        .iter()
        .map(|t| t.frontmatter.id.clone())
        .collect();

    // Re-select from all_tasks by ID (we need owned references for do_migrate)
    let tasks_to_migrate: Vec<&crate::model::task::Task> = all_tasks
        .iter()
        .filter(|t| task_ids.contains(&t.frontmatter.id))
        .collect();

    do_migrate(
        &source_ws.store_dir,
        &dest_ws.store_dir,
        &tasks_to_migrate,
        is_move,
        args.dry_run,
        args.force,
        git_commit,
    )
}

// ── Task selection by CLI args ────────────────────────────────────────────

fn select_tasks_by_args<'a>(
    all_tasks: &'a [crate::model::task::Task],
    task_ids: &[String],
    stacks: &[String],
    all: bool,
) -> Result<Vec<&'a crate::model::task::Task>> {
    if all && stacks.is_empty() && task_ids.is_empty() {
        // All tasks
        return Ok(all_tasks.iter().collect());
    }

    let mut selected: Vec<&crate::model::task::Task> = Vec::new();

    // Filter by specific task IDs
    if !task_ids.is_empty() {
        for id_prefix in task_ids {
            let matches: Vec<_> = all_tasks
                .iter()
                .filter(|t| t.frontmatter.id.starts_with(id_prefix))
                .collect();
            match matches.len() {
                0 => {
                    eprintln!("Warning: no task matching prefix '{id_prefix}'");
                }
                1 => selected.push(matches[0]),
                _ => {
                    return Err(TodoError::Other(format!(
                        "Ambiguous task prefix '{id_prefix}' — matches {} tasks. Use a longer prefix.",
                        matches.len()
                    )));
                }
            }
        }
    }

    // Filter by stack
    if !stacks.is_empty() {
        let stack_set: HashSet<&str> = stacks.iter().map(|s| s.as_str()).collect();
        for task in all_tasks {
            if let Some(ref task_stack) = task.frontmatter.stack {
                if stack_set.contains(task_stack.as_str()) {
                    // Avoid duplicates if also selected by ID
                    if !selected.iter().any(|t| t.frontmatter.id == task.frontmatter.id) {
                        selected.push(task);
                    }
                }
            }
        }
    }

    // --all with stacks means all tasks in those stacks (already handled above)
    // --all without stacks means literally all tasks (handled at the top)

    Ok(selected)
}

// ── Core migration logic ──────────────────────────────────────────────────

fn do_migrate(
    source_dir: &Path,
    dest_dir: &Path,
    tasks: &[&crate::model::task::Task],
    is_move: bool,
    dry_run: bool,
    force: bool,
    git_commit: bool,
) -> Result<()> {
    let op_str = if is_move { "Move" } else { "Copy" };
    let op_past = if is_move { "Moved" } else { "Copied" };

    // Warn about orphaned references
    let migrating_ids: HashSet<&str> = tasks.iter().map(|t| t.frontmatter.id.as_str()).collect();
    for task in tasks {
        let mut orphaned = Vec::new();

        if let Some(ref pid) = task.frontmatter.parent_id {
            if !migrating_ids.contains(pid.as_str()) {
                orphaned.push(format!("parent_id={}", &pid[..pid.len().min(10)]));
            }
        }
        for sid in &task.frontmatter.subtask_ids {
            if !migrating_ids.contains(sid.as_str()) {
                orphaned.push(format!("subtask={}", &sid[..sid.len().min(10)]));
            }
        }
        for dep in &task.frontmatter.dependencies {
            if !migrating_ids.contains(dep.task_id.as_str()) {
                orphaned.push(format!(
                    "{:?}={}",
                    dep.dep_type,
                    &dep.task_id[..dep.task_id.len().min(10)]
                ));
            }
        }

        if !orphaned.is_empty() {
            let id_prefix = &task.frontmatter.id[..task.frontmatter.id.len().min(10)];
            eprintln!(
                "Warning: {} '{}' has cross-workspace references: {}",
                id_prefix,
                task.frontmatter.title,
                orphaned.join(", ")
            );
        }
    }

    // Check for ID conflicts in destination
    let dest_store = TaskStore::with_root(dest_dir.to_path_buf());
    let dest_tasks = dest_store.load_all().unwrap_or_default();
    let dest_ids: HashSet<String> = dest_tasks
        .iter()
        .map(|t| t.frontmatter.id.clone())
        .collect();

    let mut conflicts = Vec::new();
    let mut to_migrate = Vec::new();
    for task in tasks {
        if dest_ids.contains(&task.frontmatter.id) {
            if force {
                to_migrate.push(*task);
            } else {
                conflicts.push(task.frontmatter.id.clone());
            }
        } else {
            to_migrate.push(*task);
        }
    }

    if !conflicts.is_empty() {
        let preview: Vec<String> = conflicts
            .iter()
            .map(|id| id[..id.len().min(10)].to_string())
            .collect();
        eprintln!(
            "Skipping {} task(s) due to ID conflicts in destination: {}",
            conflicts.len(),
            preview.join(", ")
        );
        eprintln!("Use --force to overwrite.");
    }

    if to_migrate.is_empty() {
        println!("No tasks to migrate after conflict resolution.");
        return Ok(());
    }

    if dry_run {
        println!();
        println!("Dry run — no changes will be made.");
        println!();
        println!(
            "{op_str} {} task(s) from {} to {}",
            to_migrate.len(),
            source_dir.display(),
            dest_dir.display()
        );
        for task in &to_migrate {
            let id_prefix = &task.frontmatter.id[..task.frontmatter.id.len().min(10)];
            let stack_str = task
                .frontmatter
                .stack
                .as_deref()
                .map(|s| format!(" @{s}"))
                .unwrap_or_default();
            println!(
                "  {} {}{} [{}]",
                id_prefix, task.frontmatter.title, stack_str, task.frontmatter.status
            );
        }
        if !conflicts.is_empty() {
            println!("  ({} skipped due to conflicts)", conflicts.len());
        }
        return Ok(());
    }

    // Pre-migration git commits
    let source_repo = if git_commit {
        pre_migrate_git(source_dir)?
    } else {
        None
    };
    let dest_repo = if git_commit {
        pre_migrate_git(dest_dir)?
    } else {
        None
    };

    // Copy tasks to destination
    let dest_manifest = ManifestStore::with_path(dest_dir.join("manifest.json"));
    for task in &to_migrate {
        dest_store.save(task)?;

        // Register stacks and tags in destination manifest
        if let Some(ref stack) = task.frontmatter.stack {
            let _ = dest_manifest.register_stack(stack);
        }
        if !task.frontmatter.tags.is_empty() {
            let _ = dest_manifest.register_tags(&task.frontmatter.tags);
        }
    }

    // If moving, delete from source
    if is_move {
        let source_store = TaskStore::with_root(source_dir.to_path_buf());
        let source_manifest = ManifestStore::with_path(source_dir.join("manifest.json"));

        for task in &to_migrate {
            if let Err(e) = source_store.delete(&task.frontmatter.id) {
                eprintln!(
                    "Warning: failed to delete {} from source: {e}",
                    &task.frontmatter.id[..task.frontmatter.id.len().min(10)]
                );
            }
        }

        // Prune source manifest
        if let Ok(remaining) = source_store.load_all() {
            let _ = source_manifest.prune_stacks_and_tags(&remaining);
        }
    }

    // Post-migration git commits
    let summary = format!(
        "migrate {} task(s) {} [{}]",
        to_migrate.len(),
        if is_move {
            format!("from {}", source_dir.display())
        } else {
            format!("to {}", dest_dir.display())
        },
        if is_move { "move" } else { "copy" }
    );

    if let Some(repo) = source_repo {
        if is_move {
            post_migrate_git(&repo, source_dir, &summary);
        }
    }
    if let Some(repo) = dest_repo {
        post_migrate_git(&repo, dest_dir, &summary);
    }

    println!(
        "{op_past} {} task(s) from {} to {}.",
        to_migrate.len(),
        source_dir.display(),
        dest_dir.display()
    );
    if !conflicts.is_empty() {
        println!("Skipped {} due to conflicts.", conflicts.len());
    }

    Ok(())
}

// ── Git helpers ───────────────────────────────────────────────────────────

/// If the store dir is inside a git repo and has changes, create a pre-migrate snapshot.
fn pre_migrate_git(store_dir: &Path) -> Result<Option<git2::Repository>> {
    let repo_root = match find_git_repo_root(store_dir) {
        Some(root) => root,
        None => return Ok(None),
    };

    let repo = match git2::Repository::open(&repo_root) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Warning: cannot open git repo at {}: {e}", repo_root.display());
            return Ok(None);
        }
    };

    // Check if store dir has changes
    let store_relative = match store_dir.strip_prefix(&repo_root) {
        Ok(rel) => rel,
        Err(_) => {
            eprintln!("Warning: store dir is not inside the git repo");
            return Ok(None);
        }
    };

    // Check if store dir has changes (scoped to drop borrow before returning repo)
    let is_dirty = {
        let mut opts = git2::StatusOptions::new();
        opts.pathspec(format!("{}/**", store_relative.display()));
        opts.include_untracked(true);

        match repo.statuses(Some(&mut opts)) {
            Ok(s) => !s.is_empty(),
            Err(e) => {
                eprintln!("Warning: git status check failed: {e}");
                false
            }
        }
    };

    if is_dirty {
        match git_stage_and_commit(&repo, store_relative, "stackydo: pre-migrate snapshot") {
            Ok(_) => {}
            Err(e) => eprintln!("Warning: pre-migrate git commit failed: {e}"),
        }
    }

    Ok(Some(repo))
}

/// Create a post-migration git commit.
fn post_migrate_git(repo: &git2::Repository, store_dir: &Path, summary: &str) {
    let repo_root = match repo.workdir() {
        Some(r) => r,
        None => return,
    };

    let store_relative = match store_dir.strip_prefix(repo_root) {
        Ok(rel) => rel,
        Err(_) => return,
    };

    let msg = format!("stackydo: {summary}");
    if let Err(e) = git_stage_and_commit(repo, store_relative, &msg) {
        eprintln!("Warning: post-migrate git commit failed: {e}");
    }
}

/// Stage files under a directory and create a commit.
fn git_stage_and_commit(
    repo: &git2::Repository,
    dir_relative: &Path,
    message: &str,
) -> std::result::Result<git2::Oid, git2::Error> {
    let mut index = repo.index()?;

    // Add all files under the directory
    let pattern = format!("{}/*", dir_relative.display());
    index.add_all([&pattern], git2::IndexAddOption::DEFAULT, None)?;

    // Also handle deleted files
    index.update_all([&pattern], None)?;

    let oid = index.write_tree()?;
    index.write()?;

    let tree = repo.find_tree(oid)?;

    let sig = repo
        .signature()
        .or_else(|_| git2::Signature::now("stackydo", "stackydo@localhost"))?;

    let parent = match repo.head() {
        Ok(head) => {
            let target = head
                .target()
                .ok_or_else(|| git2::Error::from_str("HEAD has no target"))?;
            Some(repo.find_commit(target)?)
        }
        Err(_) => None,
    };

    let parents: Vec<&git2::Commit> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
}
