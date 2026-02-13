/// Git repository context snapshot
#[derive(Debug, Clone)]
pub struct GitContext {
    pub branch: Option<String>,
    pub remote: Option<String>,
    pub commit: Option<String>,
}

/// Aggregated directory context for task creation
#[derive(Debug, Clone)]
pub struct DirectoryContext {
    pub cwd: String,
    pub git: Option<GitContext>,
}

/// Contents of a discovered .stackstodo-context file
#[derive(Debug, Clone)]
pub struct TodoContextFile {
    pub path: String,
    pub content: String,
}
