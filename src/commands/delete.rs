use crate::cli::args::DeleteArgs;
use crate::error::Result;
use crate::storage::task_store::TaskStore;

pub fn execute(args: &DeleteArgs) -> Result<()> {
    let store = TaskStore::new();
    let task = crate::commands::show::resolve_task_pub(&store, &args.id)?;

    if !args.force {
        eprint!(
            "Delete '{}' ({})? [y/N] ",
            task.frontmatter.title,
            &task.frontmatter.id[..10]
        );
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    store.delete(&task.frontmatter.id)?;
    println!("Deleted: {}", &task.frontmatter.id[..10]);

    Ok(())
}
