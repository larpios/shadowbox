mod config;
mod git;
mod ops;
mod store;

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
    #[command(alias = "ln")]
    Link {
        /// Name of the store to link to
        store: String,
    },
    /// Pull changes from the private remote store
    #[command(alias = "pl")]
    Pull,
    /// Push local changes to the private remote store
    #[command(alias = "ps", alias = "ph")]
    Push,
    /// Track a file in .shadowbox and .gitignore
    #[command(alias = "tr", alias = "t")]
    Track {
        /// Path to the file to track (supports globs)
        path: String,
    },
    /// Untrack a file by removing it from .shadowbox and .gitignore
    #[command(alias = "ut", alias = "u")]
    Untrack {
        /// Path to the file to untrack
        path: String,
    },
    /// List all tracked files
    #[command(alias = "st", alias = "s")]
    Status,
    /// Manage Git hooks for automatic syncing
    #[command(alias = "h")]
    Hooks {
        #[command(subcommand)]
        command: HookCommands,
    },
    /// Manage private stores
    #[command(alias = "sr", alias = "v")]
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

    match &cli.command {
        Some(Commands::Link { store }) => match ops::link(store) {
            Ok(_) => println!("Current project linked to store '{}'.", store),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        },
        Some(Commands::Pull) => match ops::pull() {
            Ok(_) => println!("Pull complete."),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        },
        Some(Commands::Push) => match ops::push() {
            Ok(_) => println!("Push complete."),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        },
        Some(Commands::Store { command }) => match command {
            StoreCommands::Add { name, url } => match store::add_store(name, url) {
                Ok(_) => println!("Store '{}' added.", name),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            },
            StoreCommands::List => match store::list_stores() {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            },
        },
        Some(Commands::Map { pattern, store }) => match store::add_mapping(pattern, store) {
            Ok(_) => println!("Mapped '{}' to '{}'.", pattern, store),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        },
        Some(Commands::Track { path }) => match ops::track_file(path) {
            Ok(paths) => {
                for p in paths {
                    println!("Tracked {}", p.display());
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        },
        Some(Commands::Untrack { path }) => match ops::untrack_file(path) {
            Ok(normalized_path) => {
                println!("Untracked {}", normalized_path.display());
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        },
        Some(Commands::Status) => match ops::status() {
            Ok(files) => {
                for file in files {
                    println!("{}", file);
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        },
        Some(Commands::Hooks { command }) => match command {
            HookCommands::Install => match git::install_hooks() {
                Ok(_) => println!("Hooks installed successfully."),
                Err(e) => {
                    eprintln!("Error installing hooks: {}", e);
                    std::process::exit(1);
                }
            },
            HookCommands::Uninstall => match git::uninstall_hooks() {
                Ok(_) => println!("Hooks uninstalled successfully."),
                Err(e) => {
                    eprintln!("Error uninstalling hooks: {}", e);
                    std::process::exit(1);
                }
            },
        },
        None => {
            println!("No command specified. Use --help for more info.");
        }
    }
}
