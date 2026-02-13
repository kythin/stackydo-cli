use crate::model::context::GitContext;
use std::path::Path;

/// Attempt to capture git context from a directory.
/// Returns None if the directory is not inside a git repo.
pub fn capture(path: &Path) -> Option<GitContext> {
    let repo = git2::Repository::discover(path).ok()?;

    let branch = repo
        .head()
        .ok()
        .and_then(|head| head.shorthand().map(String::from));

    let remote = repo
        .find_remote("origin")
        .ok()
        .and_then(|r| r.url().map(String::from));

    let commit = repo
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok())
        .map(|c| format!("{:.7}", c.id()));

    Some(GitContext {
        branch,
        remote,
        commit,
    })
}
