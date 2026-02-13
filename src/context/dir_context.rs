use crate::context::{git_context, todo_context};
use crate::model::task::ContextInfo;
use std::path::Path;

/// Build the full context info for a new task.
///
/// `context_path` is the user-supplied --context-path or CWD.
pub fn capture(context_path: &Path) -> ContextInfo {
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| ".".into());

    let git = git_context::capture(context_path);
    let todo_ctx = todo_context::discover(context_path);
    let session_prev = std::env::var("TODO_LAST_ID").ok();

    ContextInfo {
        path: None,
        line: None,
        column: None,
        lookfor: None,
        git_branch: git.as_ref().and_then(|g| g.branch.clone()),
        git_remote: git.as_ref().and_then(|g| g.remote.clone()),
        git_commit: git.as_ref().and_then(|g| g.commit.clone()),
        working_dir: cwd,
        todo_context_content: todo_ctx.map(|c| c.content),
        session_prev_task_id: session_prev,
    }
}
