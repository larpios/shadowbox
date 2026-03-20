use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Sync {
        #[arg(long)]
        test_mode: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Sync { test_mode }) => {
            if *test_mode {
                println!("Syncing in test mode...");
                println!("Success: true");
                println!("Time: 0ms");
            } else {
                println!("Syncing...");
            }
        }
        None => {
            println!("No command specified. Use --help for more info.");
        }
    }
}
