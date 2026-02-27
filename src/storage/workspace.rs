use crate::model::config::StackydoConfig;
use crate::storage::task_store::TaskStore;
use serde::Serialize;
use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};

/// Information about a discovered stackydo workspace.
#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceInfo {
    /// Path to stackydo.json (None for global default).
    pub config_path: Option<PathBuf>,
    /// Resolved absolute path to the task store directory.
    pub store_dir: PathBuf,
    /// Whether this is the global ~/.stackydo default.
    pub is_default: bool,
    /// Number of tasks in this workspace.
    pub task_count: usize,
    /// Unique stack names found in tasks.
    pub stacks: Vec<String>,
    /// Project name from config context.project.
    pub project_name: Option<String>,
    /// Git repo root if store_dir is inside a git repo.
    pub git_repo_root: Option<PathBuf>,
}

impl WorkspaceInfo {
    /// Human-readable label for display.
    pub fn label(&self) -> String {
        if self.is_default {
            let dir = self.store_dir.display();
            return format!("{dir}/ (global default)");
        }
        if let Some(ref name) = self.project_name {
            if let Some(ref cfg) = self.config_path {
                return format!("{} ({name})", cfg.display());
            }
        }
        if let Some(ref cfg) = self.config_path {
            return cfg.display().to_string();
        }
        format!("{}/", self.store_dir.display())
    }
}

/// Discover all stackydo workspaces accessible from the current environment.
///
/// Strategy (scoped, not full filesystem):
/// 1. Always include ~/.stackydo/ if it exists
/// 2. Walk up from CWD to root looking for stackydo.json files
/// 3. Scan well-known project directories 2 levels deep
/// 4. Deduplicate by canonicalized store_dir
pub fn discover_workspaces() -> Vec<WorkspaceInfo> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut workspaces: Vec<WorkspaceInfo> = Vec::new();

    // 1. Global default ~/.stackydo/
    let global_dir = home.join(".stackydo");
    if global_dir.is_dir() {
        if let Some(canonical) = canonicalize_safe(&global_dir) {
            seen.insert(canonical);
        }
        workspaces.push(load_workspace_info(None, global_dir, true));
    }

    // 2. Walk up from CWD looking for stackydo.json
    {
        let mut current = cwd.clone();
        loop {
            let candidate = current.join("stackydo.json");
            if candidate.is_file() {
                if let Some(info) = workspace_from_config(&candidate) {
                    if let Some(canonical) = canonicalize_safe(&info.store_dir) {
                        if seen.insert(canonical) {
                            workspaces.push(info);
                        }
                    } else {
                        workspaces.push(info);
                    }
                }
            }
            if !current.pop() {
                break;
            }
        }
    }

    // 3. Scan well-known project directories
    let well_known = [
        "Documents",
        "Projects",
        "Developer",
        "src",
        "code",
        "repos",
        "work",
    ];
    for dir_name in &well_known {
        let scan_root = home.join(dir_name);
        if !scan_root.is_dir() {
            continue;
        }
        scan_directory(&scan_root, 2, &mut seen, &mut workspaces);
    }

    // Sort: global default first, then by proximity to CWD, then alphabetical
    workspaces.sort_by(|a, b| {
        // Global default always first
        if a.is_default && !b.is_default {
            return std::cmp::Ordering::Less;
        }
        if !a.is_default && b.is_default {
            return std::cmp::Ordering::Greater;
        }

        // Then by shared path components with CWD (more = closer)
        let a_shared = shared_components(&a.store_dir, &cwd);
        let b_shared = shared_components(&b.store_dir, &cwd);
        b_shared.cmp(&a_shared).then(a.store_dir.cmp(&b.store_dir))
    });

    workspaces
}

/// Scan a directory up to `max_depth` levels deep for stackydo.json files.
fn scan_directory(
    root: &Path,
    max_depth: usize,
    seen: &mut HashSet<PathBuf>,
    workspaces: &mut Vec<WorkspaceInfo>,
) {
    let skip_dirs: HashSet<&str> = [".git", "node_modules", "target", ".stackydo"]
        .iter()
        .copied()
        .collect();

    let walker = walkdir::WalkDir::new(root)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                e.file_name()
                    .to_str()
                    .map(|s| !skip_dirs.contains(s))
                    .unwrap_or(true)
            } else {
                true
            }
        });

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue, // permission errors, etc.
        };
        if entry.file_type().is_file()
            && entry.file_name().to_str() == Some("stackydo.json")
        {
            if let Some(info) = workspace_from_config(entry.path()) {
                if let Some(canonical) = canonicalize_safe(&info.store_dir) {
                    if seen.insert(canonical) {
                        workspaces.push(info);
                    }
                } else {
                    workspaces.push(info);
                }
            }
        }
    }
}

/// Parse a stackydo.json config and build a WorkspaceInfo from it.
fn workspace_from_config(config_path: &Path) -> Option<WorkspaceInfo> {
    let content = std::fs::read_to_string(config_path).ok()?;
    let config: StackydoConfig = serde_json::from_str(&content).ok()?;

    let config_dir = config_path.parent()?;

    // Resolve store directory from config's `dir` field
    let store_dir = if let Some(ref dir_field) = config.dir {
        config_dir.join(dir_field)
    } else {
        // No dir field — this config doesn't point to a workspace store
        return None;
    };

    // Only include if the store directory exists
    if !store_dir.is_dir() {
        return None;
    }

    let project_name = config
        .context
        .as_ref()
        .and_then(|c| c.project.clone());

    let mut info = load_workspace_info(Some(config_path.to_path_buf()), store_dir, false);
    info.project_name = project_name;
    Some(info)
}

/// Load metadata for a workspace from its store directory.
fn load_workspace_info(
    config_path: Option<PathBuf>,
    store_dir: PathBuf,
    is_default: bool,
) -> WorkspaceInfo {
    let store = TaskStore::with_root(store_dir.clone());
    let (task_count, stacks) = match store.load_all() {
        Ok(tasks) => {
            let count = tasks.len();
            let mut stack_set = BTreeSet::new();
            for task in &tasks {
                if let Some(ref s) = task.frontmatter.stack {
                    stack_set.insert(s.clone());
                }
            }
            (count, stack_set.into_iter().collect())
        }
        Err(_) => (0, Vec::new()),
    };

    let git_repo_root = find_git_repo_root(&store_dir);

    WorkspaceInfo {
        config_path,
        store_dir,
        is_default,
        task_count,
        stacks,
        project_name: None,
        git_repo_root,
    }
}

/// Find the git repository root containing the given path, if any.
pub fn find_git_repo_root(path: &Path) -> Option<PathBuf> {
    git2::Repository::discover(path)
        .ok()
        .and_then(|repo| repo.workdir().map(|p| p.to_path_buf()))
}

/// Safely canonicalize a path, returning None if it fails.
fn canonicalize_safe(path: &Path) -> Option<PathBuf> {
    std::fs::canonicalize(path).ok()
}

/// Count shared leading path components between two paths.
fn shared_components(a: &Path, b: &Path) -> usize {
    a.components()
        .zip(b.components())
        .take_while(|(ac, bc)| ac == bc)
        .count()
}

/// Resolve a user-provided workspace path string to a store directory.
///
/// Accepts flexible input:
/// - Path to a stackydo.json file → parse its `dir` field, resolve relative to config location
/// - Path to a directory containing manifest.json → use directly as task store
/// - Path to a directory containing stackydo.json → parse and resolve
/// - Otherwise → assume it's a task store directory
pub fn resolve_workspace_path(input: &str) -> std::result::Result<PathBuf, String> {
    let path = PathBuf::from(shellexpand(input));

    // If it's a file named stackydo.json
    if path.is_file()
        && path
            .file_name()
            .and_then(|f| f.to_str())
            == Some("stackydo.json")
    {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
        let config: StackydoConfig = serde_json::from_str(&content)
            .map_err(|e| format!("Invalid JSON in {}: {e}", path.display()))?;
        if let Some(dir_field) = config.dir {
            let config_dir = path
                .parent()
                .ok_or_else(|| "Config file has no parent directory".to_string())?;
            let store = config_dir.join(dir_field);
            return Ok(store);
        }
        return Err(format!(
            "{} has no 'dir' field — cannot determine store location",
            path.display()
        ));
    }

    // If it's a directory
    if path.is_dir() {
        // Contains manifest.json → it's a task store
        if path.join("manifest.json").is_file() {
            return Ok(path);
        }
        // Contains stackydo.json → parse it
        let config_file = path.join("stackydo.json");
        if config_file.is_file() {
            return resolve_workspace_path(config_file.to_str().unwrap_or(""));
        }
        // Assume it's a task store directory (might not exist yet, that's OK for dest)
        return Ok(path);
    }

    // Path doesn't exist yet — might be a destination, allow it
    Ok(path)
}

/// Simple ~ expansion for paths.
fn shellexpand(input: &str) -> String {
    if let Some(rest) = input.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest).to_string_lossy().into_owned();
        }
    }
    if input == "~" {
        if let Some(home) = dirs::home_dir() {
            return home.to_string_lossy().into_owned();
        }
    }
    input.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_resolve_workspace_path_directory_with_manifest() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("manifest.json"), "{}").unwrap();
        let result = resolve_workspace_path(tmp.path().to_str().unwrap());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), tmp.path().to_path_buf());
    }

    #[test]
    fn test_resolve_workspace_path_config_file() {
        let tmp = tempfile::tempdir().unwrap();
        let config = r#"{"dir": ".tasks"}"#;
        let config_path = tmp.path().join("stackydo.json");
        fs::write(&config_path, config).unwrap();
        let result = resolve_workspace_path(config_path.to_str().unwrap());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), tmp.path().join(".tasks"));
    }

    #[test]
    fn test_resolve_workspace_path_dir_with_config() {
        let tmp = tempfile::tempdir().unwrap();
        let config = r#"{"dir": ".mystore"}"#;
        fs::write(tmp.path().join("stackydo.json"), config).unwrap();
        let result = resolve_workspace_path(tmp.path().to_str().unwrap());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), tmp.path().join(".mystore"));
    }

    #[test]
    fn test_resolve_workspace_path_nonexistent_allowed() {
        let result = resolve_workspace_path("/tmp/does-not-exist-stackydo-test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_shared_components() {
        let a = PathBuf::from("/home/user/projects/myapp/.stackydo");
        let b = PathBuf::from("/home/user/projects/myapp/src");
        // Shared: /, home, user, projects, myapp = 5 components
        assert_eq!(shared_components(&a, &b), 5);

        let c = PathBuf::from("/home/other");
        // Shared: /, home = 2 components
        assert_eq!(shared_components(&a, &c), 2);
    }

    #[test]
    fn test_shellexpand_tilde() {
        let expanded = shellexpand("~/test");
        assert!(!expanded.starts_with("~/"));
        assert!(expanded.ends_with("/test") || expanded.ends_with("\\test"));
    }

    #[test]
    fn test_shellexpand_no_tilde() {
        assert_eq!(shellexpand("/absolute/path"), "/absolute/path");
        assert_eq!(shellexpand("relative/path"), "relative/path");
    }

    #[test]
    fn test_workspace_from_config_no_dir_field() {
        let tmp = tempfile::tempdir().unwrap();
        let config = r#"{"context": {"project": "test"}}"#;
        fs::write(tmp.path().join("stackydo.json"), config).unwrap();
        let result = workspace_from_config(&tmp.path().join("stackydo.json"));
        assert!(result.is_none(), "Config without dir field should return None");
    }
}
