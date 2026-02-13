use clap::Parser;
use todo::cli::args::{Cli, Commands};
use todo::commands;
use todo::storage::paths::TodoPaths;

fn main() {
    // Ensure ~/.todos/ exists
    if let Err(e) = TodoPaths::ensure_root() {
        eprintln!("Error: cannot create ~/.todos/: {e}");
        std::process::exit(1);
    }

    let cli = Cli::parse();

    let result = match cli.command {
        // No subcommand → launch TUI
        None => todo::tui::run(),

        Some(Commands::Create(ref args)) => {
            commands::create::execute(args).map(|_| ())
        }
        Some(Commands::List(ref args)) => {
            commands::list::execute(args)
        }
        Some(Commands::Show(ref args)) => {
            commands::show::execute(args)
        }
        Some(Commands::Complete(ref args)) => {
            commands::complete::execute(args)
        }
        Some(Commands::Delete(ref args)) => {
            commands::delete::execute(args)
        }
        Some(Commands::Search(ref args)) => {
            commands::search::execute(args)
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
