use crate::context::{git_context, todo_context};
use crate::model::task::ContextInfo;
use std::path::Path;

/// Full capture result including metadata not stored on tasks (e.g. config file path).
pub struct CaptureResult {
    pub context: ContextInfo,
    /// Path to the `.stackydo-context` config file that was loaded, if any.
    pub config_file_path: Option<String>,
}

/// Build the full context info for a new task.
///
/// `context_path` is the user-supplied --context-path or CWD.
pub fn capture(context_path: &Path) -> ContextInfo {
    capture_full(context_path).context
}

/// Like `capture`, but also returns debug metadata (config file path, etc.).
pub fn capture_full(context_path: &Path) -> CaptureResult {
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| ".".into());

    let git = git_context::capture(context_path);
    let todo_ctx = todo_context::discover(context_path);
    let session_prev = std::env::var("STACKYDO_LAST_ID").ok();

    let config_file_path = todo_ctx.as_ref().map(|c| c.path.clone());

    let context = ContextInfo {
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
    };

    CaptureResult {
        context,
        config_file_path,
    }
}
