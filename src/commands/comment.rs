use crate::cli::args::CommentArgs;
use crate::commands::util::display_id;
use crate::error::Result;
use crate::model::task::Comment;
use crate::storage::task_store::TaskStore;
use chrono::Utc;

pub fn execute(args: &CommentArgs) -> Result<()> {
    let store = TaskStore::new();
    let mut task = crate::commands::show::resolve_task_pub(&store, &args.id)?;

    task.frontmatter.comments.push(Comment {
        ts: Utc::now(),
        text: args.text.clone(),
    });
    task.frontmatter.modified = Utc::now();
    store.save(&task)?;

    println!(
        "Comment added to {} — {}",
        display_id(&task.frontmatter),
        task.frontmatter.title
    );

    Ok(())
}
