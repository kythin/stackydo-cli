use crate::error::Result;
use crate::model::manifest::Manifest;
use crate::model::task::Task;
use crate::storage::paths::TodoPaths;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

/// JSON-backed manifest store at ~/.stackydo/manifest.json
pub struct ManifestStore {
    path: PathBuf,
}

impl Default for ManifestStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ManifestStore {
    pub fn new() -> Self {
        Self {
            path: TodoPaths::manifest(),
        }
    }

    pub fn with_path(path: PathBuf) -> Self {
        Self { path }
    }

    /// Load the manifest from disk, or return defaults if it doesn't exist yet.
    pub fn load(&self) -> Result<Manifest> {
        if !self.path.exists() {
            return Ok(Manifest::default());
        }
        let content = fs::read_to_string(&self.path)?;
        let manifest: Manifest = serde_json::from_str(&content)?;
        Ok(manifest)
    }

    /// Save the manifest to disk (pretty-printed JSON).
    pub fn save(&self, manifest: &Manifest) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(manifest)?;
        fs::write(&self.path, json)?;
        Ok(())
    }

    /// Register a tag in the manifest (idempotent).
    pub fn register_tags(&self, tags: &[String]) -> Result<()> {
        let mut manifest = self.load()?;
        for tag in tags {
            manifest.tags.insert(tag.clone());
        }
        self.save(&manifest)
    }

    /// Register a stack in the manifest (idempotent).
    pub fn register_stack(&self, stack: &str) -> Result<()> {
        let mut manifest = self.load()?;
        manifest.stacks.insert(stack.to_string());
        self.save(&manifest)
    }

    /// Allocate the next short ID (e.g. "SD1", "SD2"). Increments the counter
    /// and persists to disk. IDs are never recycled.
    ///
    /// Uses an exclusive file lock to prevent concurrent creates from
    /// allocating the same counter value.
    pub fn allocate_short_id(&self) -> Result<String> {
        use fs2::FileExt;

        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Lock a sibling file to serialize access to the counter
        let lock_path = self.path.with_extension("json.lock");
        let lock_file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&lock_path)?;
        lock_file.lock_exclusive()?;

        let mut manifest = self.load()?;
        let n = manifest.next_short_id;
        manifest.next_short_id = n + 1;
        self.save(&manifest)?;

        // Lock released when lock_file is dropped
        Ok(format!("SD{n}"))
    }

    /// Remove stacks and tags from the manifest that are no longer referenced
    /// by any of the given tasks. Call after deleting one or more tasks.
    pub fn prune_stacks_and_tags(&self, remaining_tasks: &[Task]) -> Result<()> {
        let mut manifest = self.load()?;

        let used_stacks: BTreeSet<&str> = remaining_tasks
            .iter()
            .filter_map(|t| t.frontmatter.stack.as_deref())
            .collect();
        let used_tags: BTreeSet<&str> = remaining_tasks
            .iter()
            .flat_map(|t| t.frontmatter.tags.iter().map(String::as_str))
            .collect();

        let before = (manifest.stacks.len(), manifest.tags.len());
        manifest.stacks.retain(|s| used_stacks.contains(s.as_str()));
        manifest.tags.retain(|t| used_tags.contains(t.as_str()));

        if (manifest.stacks.len(), manifest.tags.len()) != before {
            self.save(&manifest)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_registration_is_idempotent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = ManifestStore::with_path(dir.path().join("manifest.json"));

        let tags = vec!["backend".to_string(), "frontend".to_string()];
        store.register_tags(&tags).expect("first register");
        store
            .register_tags(&tags)
            .expect("second register (idempotent)");

        let manifest = store.load().expect("load");
        assert_eq!(manifest.tags.len(), 2);
        assert!(manifest.tags.contains("backend"));
        assert!(manifest.tags.contains("frontend"));
    }

    #[test]
    fn stack_registration_is_idempotent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = ManifestStore::with_path(dir.path().join("manifest.json"));

        store.register_stack("work").expect("first register");
        store
            .register_stack("work")
            .expect("second register (idempotent)");
        store.register_stack("personal").expect("third register");

        let manifest = store.load().expect("load");
        assert_eq!(manifest.stacks.len(), 2);
        assert!(manifest.stacks.contains("work"));
        assert!(manifest.stacks.contains("personal"));
    }

    #[test]
    fn load_returns_default_when_no_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = ManifestStore::with_path(dir.path().join("nonexistent.json"));

        let manifest = store.load().expect("load default");
        assert_eq!(manifest.version, "1.0");
        assert!(manifest.tags.is_empty());
        assert!(manifest.stacks.is_empty());
    }

    #[test]
    fn roundtrip_save_load() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = ManifestStore::with_path(dir.path().join("manifest.json"));

        let mut manifest = Manifest::default();
        manifest.tags.insert("test-tag".to_string());
        manifest.stacks.insert("test-stack".to_string());
        manifest.settings.quick_list_limit = 25;

        store.save(&manifest).expect("save");
        let loaded = store.load().expect("load");

        assert!(loaded.tags.contains("test-tag"));
        assert!(loaded.stacks.contains("test-stack"));
        assert_eq!(loaded.settings.quick_list_limit, 25);
    }

    #[test]
    fn allocate_short_id_increments() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = ManifestStore::with_path(dir.path().join("manifest.json"));

        assert_eq!(store.allocate_short_id().unwrap(), "SD1");
        assert_eq!(store.allocate_short_id().unwrap(), "SD2");
        assert_eq!(store.allocate_short_id().unwrap(), "SD3");

        let manifest = store.load().expect("load");
        assert_eq!(manifest.next_short_id, 4);
    }

    #[test]
    fn allocate_short_id_persists_across_reload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("manifest.json");

        // Allocate from first store instance
        let store1 = ManifestStore::with_path(path.clone());
        assert_eq!(store1.allocate_short_id().unwrap(), "SD1");
        assert_eq!(store1.allocate_short_id().unwrap(), "SD2");

        // Create new store instance pointing to same file
        let store2 = ManifestStore::with_path(path);
        assert_eq!(store2.allocate_short_id().unwrap(), "SD3");
    }
}
