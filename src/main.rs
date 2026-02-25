use clap::Parser;
use stackydo::cli::args::{Cli, Commands};
use stackydo::commands;
use stackydo::storage::paths::TodoPaths;

fn main() {
    // Resolve task store root from env / stackydo.json / default
    TodoPaths::init();

    // Ensure storage directory exists
    if let Err(e) = TodoPaths::ensure_root() {
        eprintln!("Error: cannot create {}: {e}", TodoPaths::root().display());
        std::process::exit(1);
    }

    let cli = Cli::parse();

    let result = match cli.command {
        None => unreachable!("clap exits before this point"),

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
        Some(Commands::Context(ref args)) => {
            commands::context::execute(args)
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
        Some(Commands::McpSetup(ref args)) => {
            commands::mcp_setup::execute(args)
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
