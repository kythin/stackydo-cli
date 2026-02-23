use crate::model::config::ResolvedConfig;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// How the task store root was resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionSource {
    /// `STACKYDO_DIR` environment variable
    Env,
    /// `dir` field in `.stackydo-context`
    Config,
    /// Default `~/.stackydo/`
    Default,
}

impl ResolutionSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResolutionSource::Env => "env",
            ResolutionSource::Config => "config",
            ResolutionSource::Default => "default",
        }
    }
}

/// Cached result of path resolution.
pub(crate) struct ResolvedPaths {
    root: PathBuf,
    source: ResolutionSource,
    config: Option<ResolvedConfig>,
}

static RESOLVED: OnceLock<ResolvedPaths> = OnceLock::new();

/// Centralized path resolution for the todo system.
pub struct TodoPaths;

impl TodoPaths {
    /// Initialize path resolution from the current working directory.
    /// Must be called once at startup, before any other `TodoPaths` method.
    /// Subsequent calls are no-ops (the `OnceLock` is already set).
    pub fn init() {
        RESOLVED.get_or_init(|| {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            Self::resolve(&cwd)
        });
    }

    /// Resolve the task store root using the priority chain:
    /// 1. `STACKYDO_DIR` env var
    /// 2. `dir` field in nearest `.stackydo-context`
    /// 3. `~/.stackydo/`
    pub(crate) fn resolve(start_dir: &Path) -> ResolvedPaths {
        use crate::context::todo_context;

        // Priority 1: env var
        if let Ok(dir) = std::env::var("STACKYDO_DIR") {
            let config = todo_context::discover_config(start_dir);
            return ResolvedPaths {
                root: PathBuf::from(dir),
                source: ResolutionSource::Env,
                config,
            };
        }

        // Priority 2: config file `dir` field
        if let Some(resolved_config) = todo_context::discover_config(start_dir) {
            if let Some(ref dir_field) = resolved_config.config.dir {
                // Resolve relative to the config file's parent directory
                let config_dir = resolved_config
                    .file_path
                    .parent()
                    .expect("config file must have a parent directory");
                let root = config_dir.join(dir_field);
                return ResolvedPaths {
                    root,
                    source: ResolutionSource::Config,
                    config: Some(resolved_config),
                };
            }
            // Config exists but no `dir` field — fall through to default
            return ResolvedPaths {
                root: Self::default_root(),
                source: ResolutionSource::Default,
                config: Some(resolved_config),
            };
        }

        // Priority 3: default
        ResolvedPaths {
            root: Self::default_root(),
            source: ResolutionSource::Default,
            config: None,
        }
    }

    fn default_root() -> PathBuf {
        dirs::home_dir()
            .expect("Cannot determine home directory")
            .join(".stackydo")
    }

    /// Root storage directory.
    /// If `init()` was called, uses the cached resolved path.
    /// Otherwise falls back to env var / default (preserving pre-init behavior).
    pub fn root() -> PathBuf {
        if let Some(resolved) = RESOLVED.get() {
            resolved.root.clone()
        } else if let Ok(dir) = std::env::var("STACKYDO_DIR") {
            PathBuf::from(dir)
        } else {
            Self::default_root()
        }
    }

    /// How the root was resolved: "env", "config", or "default".
    pub fn resolution_source() -> ResolutionSource {
        RESOLVED
            .get()
            .map(|r| r.source.clone())
            .unwrap_or(if std::env::var("STACKYDO_DIR").is_ok() {
                ResolutionSource::Env
            } else {
                ResolutionSource::Default
            })
    }

    /// The resolved config from init, if any.
    pub fn resolved_config() -> Option<&'static ResolvedConfig> {
        RESOLVED.get().and_then(|r| r.config.as_ref())
    }

    /// Path to the manifest file
    pub fn manifest() -> PathBuf {
        Self::root().join("manifest.json")
    }

    /// Path to a task markdown file by ULID
    pub fn task_file(task_id: &str) -> PathBuf {
        Self::root().join(format!("{task_id}.md"))
    }

    /// Ensure the root storage directory exists
    pub fn ensure_root() -> std::io::Result<()> {
        std::fs::create_dir_all(Self::root())
    }

    /// Walk up from `start_dir` looking for a `.stackydo-context` file.
    /// Returns the first one found, or None.
    pub fn find_todo_context(start_dir: &Path) -> Option<PathBuf> {
        let mut current = start_dir.to_path_buf();
        loop {
            let candidate = current.join(".stackydo-context");
            if candidate.is_file() {
                return Some(candidate);
            }
            if !current.pop() {
                break;
            }
        }
        None
    }

    /// Fallback ~/.stackydo-context path
    pub fn home_todo_context() -> PathBuf {
        dirs::home_dir()
            .expect("Cannot determine home directory")
            .join(".stackydo-context")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_resolve_with_config_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let config_content = "dir: ./my-workspace\nproject: test\n";
        fs::write(tmp.path().join(".stackydo-context"), config_content).unwrap();

        // Temporarily unset STACKYDO_DIR to test config resolution
        let old_env = std::env::var("STACKYDO_DIR").ok();
        std::env::remove_var("STACKYDO_DIR");

        let resolved = TodoPaths::resolve(tmp.path());
        assert_eq!(resolved.source, ResolutionSource::Config);
        assert_eq!(resolved.root, tmp.path().join("./my-workspace"));
        assert!(resolved.config.is_some());
        let cfg = resolved.config.unwrap();
        assert_eq!(cfg.config.dir.as_deref(), Some("./my-workspace"));
        assert_eq!(cfg.config.project.as_deref(), Some("test"));

        // Restore env
        if let Some(val) = old_env {
            std::env::set_var("STACKYDO_DIR", val);
        }
    }

    #[test]
    fn test_resolve_env_overrides_config() {
        let tmp = tempfile::tempdir().unwrap();
        let config_content = "dir: ./my-workspace\n";
        fs::write(tmp.path().join(".stackydo-context"), config_content).unwrap();

        let old_env = std::env::var("STACKYDO_DIR").ok();
        std::env::set_var("STACKYDO_DIR", "/tmp/env-override");

        let resolved = TodoPaths::resolve(tmp.path());
        assert_eq!(resolved.source, ResolutionSource::Env);
        assert_eq!(resolved.root, PathBuf::from("/tmp/env-override"));

        // Restore env
        match old_env {
            Some(val) => std::env::set_var("STACKYDO_DIR", val),
            None => std::env::remove_var("STACKYDO_DIR"),
        }
    }

    #[test]
    fn test_resolve_default_no_config() {
        let tmp = tempfile::tempdir().unwrap();
        // No .stackydo-context file

        let old_env = std::env::var("STACKYDO_DIR").ok();
        std::env::remove_var("STACKYDO_DIR");

        let resolved = TodoPaths::resolve(tmp.path());
        assert_eq!(resolved.source, ResolutionSource::Default);
        assert!(resolved.config.is_none());

        if let Some(val) = old_env {
            std::env::set_var("STACKYDO_DIR", val);
        }
    }

    #[test]
    fn test_resolve_config_without_dir_field() {
        let tmp = tempfile::tempdir().unwrap();
        let config_content = "project: test\nstack: dev\n";
        fs::write(tmp.path().join(".stackydo-context"), config_content).unwrap();

        let old_env = std::env::var("STACKYDO_DIR").ok();
        std::env::remove_var("STACKYDO_DIR");

        let resolved = TodoPaths::resolve(tmp.path());
        assert_eq!(resolved.source, ResolutionSource::Default);
        assert!(resolved.config.is_some());
        assert!(resolved.config.unwrap().config.dir.is_none());

        if let Some(val) = old_env {
            std::env::set_var("STACKYDO_DIR", val);
        }
    }

    #[test]
    fn test_resolve_freeform_text_config() {
        let tmp = tempfile::tempdir().unwrap();
        // Not valid YAML key-value pairs — just freeform text
        fs::write(
            tmp.path().join(".stackydo-context"),
            "This is just some notes about the project.\nNo YAML here.",
        )
        .unwrap();

        let old_env = std::env::var("STACKYDO_DIR").ok();
        std::env::remove_var("STACKYDO_DIR");

        let resolved = TodoPaths::resolve(tmp.path());
        // Freeform text → config with all None fields → default root
        assert_eq!(resolved.source, ResolutionSource::Default);
        assert!(resolved.config.is_some());
        let cfg = resolved.config.unwrap();
        assert!(cfg.config.dir.is_none());
        assert!(cfg.raw_content.contains("just some notes"));

        if let Some(val) = old_env {
            std::env::set_var("STACKYDO_DIR", val);
        }
    }
}
