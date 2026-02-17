use clap::Parser;
use stackstodo::cli::args::{Cli, Commands};
use stackstodo::commands;
use stackstodo::storage::paths::TodoPaths;

fn main() {
    // Ensure storage directory exists
    if let Err(e) = TodoPaths::ensure_root() {
        eprintln!("Error: cannot create {}: {e}", TodoPaths::root().display());
        std::process::exit(1);
    }

    let cli = Cli::parse();

    let result = match cli.command {
        // No subcommand → launch TUI
        None => stackstodo::tui::run(),

        Some(Commands::Create(ref args)) => {
            commands::create::execute(args).map(|_| ())
        }
        Some(Commands::List(ref args)) => {
            commands::list::execute(args)
        }
        Some(Commands::Show(ref args)) => {
            commands::show::execute(args)
        }
        Some(Commands::Update(ref args)) => {
            commands::update::execute(args)
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
        Some(Commands::Context) => {
            commands::context::execute()
        }
        Some(Commands::Stats(ref args)) => {
            commands::stats::execute(args)
        }
        Some(Commands::Stacks(ref args)) => {
            commands::stacks::execute(args)
        }
        Some(Commands::Init(ref args)) => {
            commands::init::execute(args)
        }
        Some(Commands::Import(ref args)) => {
            commands::import::execute(args)
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
