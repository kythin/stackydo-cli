use crate::error::Result;
use crate::model::manifest::Manifest;
use crate::storage::paths::TodoPaths;
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
        store.register_tags(&tags).expect("second register (idempotent)");

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
        store.register_stack("work").expect("second register (idempotent)");
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
}
