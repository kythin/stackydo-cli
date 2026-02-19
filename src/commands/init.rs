use crate::cli::args::InitArgs;
use crate::error::Result;
use crate::model::manifest::Manifest;
use crate::storage::manifest_store::ManifestStore;
use std::fs;
use std::path::PathBuf;

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

    // 3. Create .stackydo-context template
    let context_path = root.join(".stackydo-context");
    if !context_path.exists() {
        let should_create = if args.yes {
            true
        } else {
            print!("Create .stackydo-context template? [Y/n] ");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let input = input.trim().to_lowercase();
            input.is_empty() || input == "y" || input == "yes"
        };

        if should_create {
            fs::write(
                &context_path,
                "# Stackydo context\n# Lines here are captured as context for new tasks.\n",
            )?;
            created.push(format!("Created context template: {}", context_path.display()));
        }
    }

    // 4. Git init if requested
    if args.git {
        let git_dir = root.join(".git");
        if git_dir.exists() {
            created.push("Git already initialized.".to_string());
        } else {
            git2::Repository::init(&root)?;
            // Create .gitignore
            let gitignore_path = root.join(".gitignore");
            if !gitignore_path.exists() {
                fs::write(&gitignore_path, "# stackydo gitignore\n")?;
            }
            created.push("Initialized git repository.".to_string());
        }
    }

    // 5. Print summary
    println!("Stackydo workspace initialized:");
    for line in &created {
        println!("  {line}");
    }

    // 6. Hint about STACKYDO_DIR if custom dir
    if args.dir.is_some() {
        println!("\nTo use this workspace, set:");
        println!("  export STACKYDO_DIR=\"{}\"", root.display());
    }

    // Check if we're in a git repo and suggest submodule approach
    if let Ok(repo) = git2::Repository::discover(".") {
        if let Some(workdir) = repo.workdir() {
            if root != workdir.join(".stackydo") {
                println!("\nTip: You can track your tasks as a git submodule:");
                println!(
                    "  git submodule add <remote-url> {}",
                    root.display()
                );
            }
        }
    }

    Ok(())
}
