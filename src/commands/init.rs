use crate::cli::args::InitArgs;
use crate::error::Result;
use crate::model::config::StackydoConfig;
use crate::model::manifest::Manifest;
use crate::storage::manifest_store::ManifestStore;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

/// Minimal template written inside the workspace directory itself.
/// References the schema so editors can provide field completion/documentation.
const WORKSPACE_CONFIG_TEMPLATE: &str = concat!(
    "{\n",
    "  \"$schema\": \"https://raw.githubusercontent.com/kythin/stackydo-cli/main/schemas/stackydo.schema.json\"\n",
    "}\n"
);

/// Project-level `stackydo.json` template written in CWD when `--here` is used.
fn project_config_template(dir: &str) -> String {
    format!(
        "{{\n  \"$schema\": \"https://raw.githubusercontent.com/kythin/stackydo-cli/main/schemas/stackydo.schema.json\",\n  \"dir\": \"{dir}\"\n}}\n"
    )
}

pub fn execute(args: &InitArgs) -> Result<()> {
    let root = if let Some(ref dir) = args.dir {
        PathBuf::from(dir)
    } else {
        crate::storage::paths::TodoPaths::root()
    };

    let mut created: Vec<String> = Vec::new();

    // 1. Create storage directory
    if !root.exists() {
        fs::create_dir_all(&root)?;
        created.push(format!("Created directory: {}", root.display()));
    } else {
        created.push(format!("Directory exists: {}", root.display()));
    }

    // 2. Write default manifest.json if absent
    let manifest_path = root.join("manifest.json");
    if !manifest_path.exists() {
        let manifest_store = ManifestStore::with_path(manifest_path.clone());
        manifest_store.save(&Manifest::default())?;
        created.push(format!("Created manifest: {}", manifest_path.display()));
    } else {
        created.push(format!("Manifest exists: {}", manifest_path.display()));
    }

    // 3. Create stackydo.json template inside the workspace
    let config_path = root.join("stackydo.json");
    if !config_path.exists() {
        let should_create = if args.yes {
            true
        } else {
            print!("Create stackydo.json config template? [Y/n] ");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let input = input.trim().to_lowercase();
            input.is_empty() || input == "y" || input == "yes"
        };

        if should_create {
            fs::write(&config_path, WORKSPACE_CONFIG_TEMPLATE)?;
            created.push(format!("Created config: {}", config_path.display()));
        }
    }

    // 4. Git init if requested
    if args.git {
        let git_dir = root.join(".git");
        if git_dir.exists() {
            created.push("Git already initialized in workspace.".to_string());
        } else {
            git2::Repository::init(&root)?;
            let gitignore_path = root.join(".gitignore");
            if !gitignore_path.exists() {
                fs::write(&gitignore_path, "# stackydo workspace\n")?;
            }
            created.push(format!("Initialized git repository: {}", root.display()));
        }

        // Add the workspace to the parent repo's .gitignore so the workspace's
        // own git history isn't tracked by the project repo.
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        if let Ok(parent_repo) = git2::Repository::discover(&cwd) {
            if let Some(workdir) = parent_repo.workdir() {
                // Resolve absolute paths; use cwd.join(root) as fallback for
                // relative paths before the dir existed.
                let abs_root = root
                    .canonicalize()
                    .unwrap_or_else(|_| cwd.join(&root));
                let abs_workdir = workdir
                    .canonicalize()
                    .unwrap_or_else(|_| workdir.to_path_buf());

                // Only act if the workspace lives inside the parent repo.
                if abs_root.starts_with(&abs_workdir) && abs_root != abs_workdir {
                    let rel = abs_root.strip_prefix(&abs_workdir).unwrap();
                    // Anchor the entry to the repo root with a leading slash.
                    let entry = format!("/{}", rel.display());
                    let rel_str = rel.to_string_lossy();

                    let parent_gitignore = abs_workdir.join(".gitignore");
                    let existing = if parent_gitignore.exists() {
                        fs::read_to_string(&parent_gitignore).unwrap_or_default()
                    } else {
                        String::new()
                    };

                    // Check for the entry in any common form.
                    let already_ignored = existing.lines().any(|l| {
                        let l = l.trim();
                        l == rel_str.as_ref()
                            || l == entry.as_str()
                            || l == format!("{rel_str}/")
                            || l == format!("{entry}/")
                    });

                    if already_ignored {
                        created.push(format!("'{rel_str}' already in parent .gitignore"));
                    } else {
                        let line = if existing.is_empty() || existing.ends_with('\n') {
                            format!("{entry}\n")
                        } else {
                            format!("\n{entry}\n")
                        };
                        fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&parent_gitignore)?
                            .write_all(line.as_bytes())?;
                        created.push(format!(
                            "Added '{entry}' to {}",
                            parent_gitignore.display()
                        ));
                    }
                }
            }
        }
    }

    // 5. Write stackydo.json in CWD when --here is passed
    if args.here {
        let dir_value = args.dir.as_deref().unwrap_or(".stackydo");
        let cwd_config_path = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("stackydo.json");

        if cwd_config_path.exists() {
            // Parse existing file, update/add the dir field, and re-write
            let existing = fs::read_to_string(&cwd_config_path)?;
            let mut config: StackydoConfig =
                serde_json::from_str(&existing).unwrap_or_default();
            config.dir = Some(dir_value.to_string());
            let new_content = serde_json::to_string_pretty(&config)
                .unwrap_or_else(|_| format!("{{\"dir\": \"{dir_value}\"}}\n"));
            fs::write(&cwd_config_path, format!("{new_content}\n"))?;
            created.push(format!("Updated stackydo.json: dir = {dir_value}"));
        } else {
            let content = project_config_template(dir_value);
            fs::write(&cwd_config_path, content)?;
            created.push(format!("Created stackydo.json: dir = {dir_value}"));
        }
    }

    // 6. Print summary
    println!("Stackydo workspace initialized:");
    for line in &created {
        println!("  {line}");
    }

    // 7. Hint about how to use the workspace
    if args.dir.is_some() && !args.here {
        println!("\nTo use this workspace, either:");
        println!("  1. Run `stackydo init --here --dir {}` to write a stackydo.json", root.display());
        println!("  2. export STACKYDO_DIR=\"{}\"  (per-session override)", root.display());
    }

    // Suggest submodule approach only when --git wasn't used.
    if !args.git {
        if let Ok(repo) = git2::Repository::discover(".") {
            if let Some(workdir) = repo.workdir() {
                if root != workdir.join(".stackydo") {
                    println!("\nTip: Use --git to initialise the workspace as its own git repo");
                    println!("     and automatically add it to the parent .gitignore, or track");
                    println!("     tasks as a git submodule:");
                    println!("       git submodule add <remote-url> {}", root.display());
                }
            }
        }
    }

    Ok(())
}
