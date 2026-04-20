mod commands;
mod config;
mod git;
mod utils;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Link the current repository to a specific store
    #[command(alias = "ln", alias = "l")]
    Link {
        /// Name of the store to link to
        store: String,
    },
    /// Pull changes from the private remote store
    #[command(alias = "pl")]
    Pull,
    /// Push local changes to the private remote store
    #[command(alias = "ps")]
    Push,
    /// Track a file or directory in the private vault
    #[command(alias = "tr", alias = "t")]
    Track {
        /// A list of patterns to the files to track (supports globs)
        patterns: Vec<String>,
        /// Follow symlinks (copy the target instead of the link)
        #[arg(long, short)]
        follow_links: bool,
    },
    /// Untrack a file or directory by removing it from the vault
    #[command(alias = "ut", alias = "u")]
    Untrack {
        /// Path to the file to untrack
        path: String,
    },
    /// List all tracked files
    #[command(alias = "st")]
    Status,
    /// Manage Git hooks for automatic syncing
    #[command(alias = "h")]
    Hooks {
        #[command(subcommand)]
        command: HookCommands,
    },
    /// Manage private stores
    #[command(alias = "s", alias = "v")]
    Store {
        #[command(subcommand)]
        command: StoreCommands,
    },
    /// Manage project mappings
    #[command(alias = "m")]
    Map {
        /// Pattern (e.g. github.com/user/*)
        pattern: String,
        /// Store name
        store: String,
    },
    /// Show version information
    Version,
    /// Diagnostic information for troubleshooting
    Debug,
}

#[derive(Subcommand)]
enum StoreCommands {
    /// Add a new private store
    #[command(alias = "a")]
    Add {
        /// Name of the store
        name: String,
        /// Git URL of the private repository
        url: String,
        /// Overwrite the store if it already exists
        #[arg(long, short)]
        force: bool,
    },
    /// List all configured stores
    #[command(alias = "ls", alias = "l")]
    List,
}

#[derive(Subcommand)]
enum HookCommands {
    /// Install Git hooks to automate syncing
    #[command(alias = "i", alias = "in")]
    Install,
    /// Uninstall Git hooks
    #[command(alias = "un")]
    Uninstall,
}

fn main() {
    let cli = Cli::parse();

    let result = match &cli.command {
        Some(Commands::Link { store }) => commands::link::run(store),
        Some(Commands::Pull) => commands::pull::run(),
        Some(Commands::Push) => commands::push::run(),
        Some(Commands::Store { command }) => match command {
            StoreCommands::Add { name, url, force } => commands::store::add(name, url, *force),
            StoreCommands::List => commands::store::list(),
        },
        Some(Commands::Map { pattern, store }) => commands::store::add_mapping(pattern, store),
        Some(Commands::Track {
            patterns,
            follow_links,
        }) => commands::track::run(patterns, *follow_links),
        Some(Commands::Untrack { path }) => commands::untrack::run(path),
        Some(Commands::Status) => commands::status::run(),
        Some(Commands::Hooks { command }) => match command {
            HookCommands::Install => commands::hooks::install(),
            HookCommands::Uninstall => commands::hooks::uninstall(),
        },
        Some(Commands::Version) => {
            println!("Shadowbox v{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some(Commands::Debug) => commands::debug::run(),
        None => {
            println!("No command specified. Use --help for more info.");
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    #[test]
    fn test_help_output() {
        let output = Command::new("cargo")
            .args(["run", "--", "--help"])
            .output()
            .expect("Failed to execute command");

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Usage: shadowbox"));
    }
}
