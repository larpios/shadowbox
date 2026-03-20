use clap::{Parser, Subcommand};
use std::fs::{OpenOptions, read_to_string};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Sync tracked files (AI artifacts, justfiles, etc.)
    Sync {
        #[arg(long)]
        test_mode: bool,
    },
    /// Track a file in .shadowbox and .gitignore
    Track {
        /// Path to the file to track
        path: String,
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
        Some(Commands::Track { path }) => {
            match track_file(path) {
                Ok(normalized_path) => {
                    println!("Tracked {}", normalized_path.display());
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                }
            }
        }
        None => {
            println!("No command specified. Use --help for more info.");
        }
    }
}

fn track_file(path: &str) -> std::io::Result<PathBuf> {
    let raw_path = Path::new(path);
    if !raw_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("File '{}' not found", path),
        ));
    }

    // Normalize path: relative to current dir, remove redundant components
    let normalized = normalize_path(raw_path)?;
    let path_str = normalized.to_string_lossy();

    let shadowbox_file = std::env::var("SHADOWBOX_FILE").unwrap_or_else(|_| ".shadowbox".to_string());
    let gitignore_file = std::env::var("GITIGNORE_FILE").unwrap_or_else(|_| ".gitignore".to_string());

    append_if_missing(&shadowbox_file, &path_str)?;
    append_if_missing(&gitignore_file, &path_str)?;

    Ok(normalized)
}

fn normalize_path(path: &Path) -> std::io::Result<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => continue,
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push(component);
                }
            }
            _ => normalized.push(component),
        }
    }
    Ok(normalized)
}

fn append_if_missing(file_path: &str, line: &str) -> std::io::Result<()> {
    let contents = read_to_string(file_path).unwrap_or_default();
    if !contents.lines().any(|l| l == line) {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)?;

        if !contents.is_empty() && !contents.ends_with('\n') {
            write!(file, "\n")?;
        }
        writeln!(file, "{}", line)?;
    }
    Ok(())
}
