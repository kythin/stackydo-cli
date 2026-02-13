use thiserror::Error;

#[derive(Error, Debug)]
pub enum TodoError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("Invalid datetime: {0}")]
    InvalidDateTime(String),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Invalid context path: {0}")]
    InvalidContextPath(String),

    #[error("Frontmatter parse error: {0}")]
    FrontmatterParse(String),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, TodoError>;
