use crate::cli::args::ContextArgs;
use crate::commands::util::print_json;
use crate::context::dir_context;
use crate::error::Result;
use crate::storage::paths::TodoPaths;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Serialize)]
struct ContextOutput {
    working_dir: String,
    git_branch: Option<String>,
    git_remote: Option<String>,
    git_commit: Option<String>,
    session_prev_task_id: Option<String>,
    config_file: Option<String>,
    config_content: Option<String>,
    task_store: String,
    manifest: String,
    resolution_source: String,
    config_dir_field: Option<String>,
    config_stack_filter: Option<String>,
    env: BTreeMap<String, Option<String>>,
}

fn collect_env_vars() -> BTreeMap<String, Option<String>> {
    let mut env = BTreeMap::new();

    // Always include the two known vars
    env.insert(
        "STACKYDO_DIR".to_string(),
        std::env::var("STACKYDO_DIR").ok(),
    );
    env.insert(
        "STACKYDO_LAST_ID".to_string(),
        std::env::var("STACKYDO_LAST_ID").ok(),
    );

    // Pick up any other STACKYDO_* vars
    for (key, value) in std::env::vars() {
        if key.starts_with("STACKYDO_") && !env.contains_key(&key) {
            env.insert(key, Some(value));
        }
    }

    env
}

/// Execute the headless `context` command: print the context metadata
/// that would be captured if a task were created from the current CWD.
pub fn execute(args: &ContextArgs) -> Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let result = dir_context::capture_full(&cwd);
    let ctx = &result.context;
    let env = collect_env_vars();

    let source = TodoPaths::resolution_source();
    let config_dir_field = TodoPaths::resolved_config()
        .and_then(|c| c.config.dir.clone());
    let config_stack_filter = TodoPaths::resolved_config()
        .and_then(|c| c.config.stack_filter.clone());

    if args.json {
        let output = ContextOutput {
            working_dir: ctx.working_dir.clone(),
            git_branch: ctx.git_branch.clone(),
            git_remote: ctx.git_remote.clone(),
            git_commit: ctx.git_commit.clone(),
            session_prev_task_id: ctx.session_prev_task_id.clone(),
            config_file: result.config_file_path.clone(),
            config_content: ctx.todo_context_content.clone(),
            task_store: TodoPaths::root().display().to_string(),
            manifest: TodoPaths::manifest().display().to_string(),
            resolution_source: source.as_str().to_string(),
            config_dir_field,
            config_stack_filter,
            env,
        };
        return print_json(&output);
    }

    println!("Context (would be captured on `stackydo create`):");
    println!();
    println!("  Working dir:     {}", ctx.working_dir);

    if let Some(ref branch) = ctx.git_branch {
        println!("  Git branch:      {branch}");
    } else {
        println!("  Git branch:      (none)");
    }
    if let Some(ref remote) = ctx.git_remote {
        println!("  Git remote:      {remote}");
    }
    if let Some(ref commit) = ctx.git_commit {
        println!("  Git commit:      {commit}");
    }

    if let Some(ref prev) = ctx.session_prev_task_id {
        println!("  Session prev ID: {prev}");
    }

    println!();
    println!("  Config file:     {}", result.config_file_path
        .as_deref()
        .unwrap_or("(no stackydo.json found)"));

    if let Some(ref content) = ctx.todo_context_content {
        println!("  Config content:");
        for line in content.lines() {
            println!("    {line}");
        }
    }

    println!();
    println!("  Task store:      {}", TodoPaths::root().display());
    println!("  Manifest:        {}", TodoPaths::manifest().display());

    println!();
    println!("  Resolution:");
    println!("    Source:         {}", source.as_str());
    if let Some(ref dir_field) = config_dir_field {
        println!("    Config dir:     {dir_field}");
    }
    if let Some(ref filter) = config_stack_filter {
        println!("    Stack filter:   {filter}");
    }
    match source {
        crate::storage::paths::ResolutionSource::Env => {
            if let Ok(val) = std::env::var("STACKYDO_DIR") {
                println!("    STACKYDO_DIR:   {val}");
            }
        }
        crate::storage::paths::ResolutionSource::Config => {
            if let Some(cfg) = TodoPaths::resolved_config() {
                println!("    From:           {}", cfg.file_path.display());
            }
        }
        crate::storage::paths::ResolutionSource::Default => {
            println!("    (no override — using ~/.stackydo/)");
        }
    }

    println!();
    println!("  Environment:");
    for (key, value) in &env {
        let display = value.as_deref().unwrap_or("(not set)");
        println!("    {key:18}{display}");
    }

    Ok(())
}
