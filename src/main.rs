use clap::{Parser, Subcommand};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions, read_to_string};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

#[derive(Serialize, Deserialize, Default)]
struct Config {
    #[serde(default)]
    stores: HashMap<String, StoreConfig>,
    #[serde(default)]
    mappings: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
struct StoreConfig {
    url: String,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize Shadowbox
    Init,
    /// Sync tracked files from the mapped store (Pull and Update .gitignore)
    Sync,
    /// Pull changes from the private remote store
    Pull,
    /// Push local changes to the private remote store
    Push,
    /// Track a file in .shadowbox and .gitignore
    Track {
        /// Path to the file to track
        path: String,
    },
    /// Untrack a file by removing it from .shadowbox and .gitignore
    Untrack {
        /// Path to the file to untrack
        path: String,
    },
    /// List all tracked files
    Status,
    /// Manage Git hooks for automatic syncing
    Hooks {
        #[command(subcommand)]
        command: HookCommands,
    },
    /// Manage private stores
    Store {
        #[command(subcommand)]
        command: StoreCommands,
    },
    /// Manage project mappings
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
    Add {
        /// Name of the store
        name: String,
        /// Git URL of the private repository
        url: String,
    },
    /// List all configured stores
    List,
}

#[derive(Subcommand)]
enum HookCommands {
    /// Install Git hooks to automate syncing
    Install,
    /// Uninstall Git hooks
    Uninstall,
}

impl Config {
    fn load() -> std::io::Result<Self> {
        let config_path = config_dir()?.join("config.toml");
        if config_path.exists() {
            let content = fs::read_to_string(config_path)?;
            Ok(toml::from_str(&content).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
            })?)
        } else {
            Ok(Config::default())
        }
    }

    fn save(&self) -> std::io::Result<()> {
        let dir = config_dir()?;
        fs::create_dir_all(&dir)?;
        let config_path = dir.join("config.toml");
        let content = toml::to_string(self).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })?;
        fs::write(config_path, content)
    }
}

fn config_dir() -> std::io::Result<PathBuf> {
    ProjectDirs::from("com", "shadowbox", "shadowbox")
        .map(|d| d.config_dir().to_path_buf())
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Could not find config directory")
        })
}

fn data_dir() -> std::io::Result<PathBuf> {
    ProjectDirs::from("com", "shadowbox", "shadowbox")
        .map(|d| d.data_dir().to_path_buf())
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Could not find data directory")
        })
}
fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Init) => match init() {
            Ok(_) => {}
            Err(e) => eprintln!("Error: {}", e),
        },
        Some(Commands::Sync) => {
            match sync() {
                Ok(_) => println!("Sync complete."),
                Err(e) => eprintln!("Error during sync: {}", e),
            }
        }
        Some(Commands::Pull) => match pull() {
            Ok(_) => println!("Pull complete."),
            Err(e) => eprintln!("Error during pull: {}", e),
        },
        Some(Commands::Push) => match push() {
            Ok(_) => println!("Push complete."),
            Err(e) => eprintln!("Error during push: {}", e),
        },
        Some(Commands::Store { command }) => match command {
            StoreCommands::Add { name, url } => match add_store(name, url) {
                Ok(_) => println!("Store '{}' added.", name),
                Err(e) => eprintln!("Error: {}", e),
            },
            StoreCommands::List => match list_stores() {
                Ok(_) => {}
                Err(e) => eprintln!("Error: {}", e),
            },
        },
        Some(Commands::Map { pattern, store }) => match add_mapping(pattern, store) {
            Ok(_) => println!("Mapped '{}' to '{}'.", pattern, store),
            Err(e) => eprintln!("Error: {}", e),
        },
        Some(Commands::Track { path }) => match track_file(path) {
            Ok(normalized_path) => {
                println!("Tracked {}", normalized_path.display());
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        },
        Some(Commands::Untrack { path }) => match untrack_file(path) {
            Ok(normalized_path) => {
                println!("Untracked {}", normalized_path.display());
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        },
        Some(Commands::Status) => match status() {
            Ok(files) => {
                for file in files {
                    println!("{}", file);
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        },
        Some(Commands::Hooks { command }) => match command {
            HookCommands::Install => match install_hooks() {
                Ok(_) => println!("Hooks installed successfully."),
                Err(e) => eprintln!("Error installing hooks: {}", e),
            },
            HookCommands::Uninstall => match uninstall_hooks() {
                Ok(_) => println!("Hooks uninstalled successfully."),
                Err(e) => eprintln!("Error uninstalling hooks: {}", e),
            },
        },
        None => {
            println!("No command specified. Use --help for more info.");
        }
    }
}

fn init() -> std::io::Result<()> {
    let shadowbox_file = std::env::var("SHADOWBOX_FILE").unwrap_or(".shadowbox".to_string());
    let gitignore_file = std::env::var("GITIGNORE_FILE").unwrap_or(".gitignore".to_string());

    if !Path::new(&shadowbox_file).exists() {
        OpenOptions::new()
            .create(true)
            .write(true)
            .open(&shadowbox_file)?;
        println!("Initialized {}", shadowbox_file);
    } else {
        println!("{} already exists", shadowbox_file);
    }

    if !Path::new(&gitignore_file).exists() {
        OpenOptions::new()
            .create(true)
            .write(true)
            .open(&gitignore_file)?;
        println!("Initialized {}", gitignore_file);
    } else {
        println!("{} already exists", gitignore_file);
    }

    Ok(())
}

fn sync() -> std::io::Result<()> {
    let shadowbox_file = std::env::var("SHADOWBOX_FILE").unwrap_or(".shadowbox".to_string());
    let gitignore_file = std::env::var("GITIGNORE_FILE").unwrap_or(".gitignore".to_string());

    let contents = read_to_string(&shadowbox_file).unwrap_or_default();
    for line in contents.lines() {
        if line.is_empty() {
            continue;
        }
        append_if_missing(&gitignore_file, line)?;
    }
    Ok(())
}

fn status() -> std::io::Result<Vec<String>> {
    let shadowbox_file = std::env::var("SHADOWBOX_FILE").unwrap_or(".shadowbox".to_string());
    let contents = read_to_string(shadowbox_file).unwrap_or_default();
    let mut results = Vec::new();
    for line in contents.lines() {
        if line.is_empty() {
            continue;
        }
        if Path::new(line).exists() {
            results.push(line.to_string());
        } else {
            results.push(format!("[MISSING] {}", line));
        }
    }
    Ok(results)
}

fn track_file(path: &str) -> std::io::Result<PathBuf> {
    let raw_path = Path::new(path);
    if !raw_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("File '{}' not found", path),
        ));
    }

    let normalized = normalize_path(raw_path)?;
    let path_str = normalized.to_string_lossy();

    let shadowbox_file = std::env::var("SHADOWBOX_FILE").unwrap_or(".shadowbox".to_string());
    let gitignore_file = std::env::var("GITIGNORE_FILE").unwrap_or(".gitignore".to_string());

    append_if_missing(&shadowbox_file, &path_str)?;
    append_if_missing(&gitignore_file, &path_str)?;

    Ok(normalized)
}

fn untrack_file(path: &str) -> std::io::Result<PathBuf> {
    let raw_path = Path::new(path);
    let normalized = normalize_path(raw_path)?;
    let path_str = normalized.to_string_lossy();

    let shadowbox_file = std::env::var("SHADOWBOX_FILE").unwrap_or(".shadowbox".to_string());
    let gitignore_file = std::env::var("GITIGNORE_FILE").unwrap_or(".gitignore".to_string());

    remove_line(&shadowbox_file, &path_str)?;
    remove_line(&gitignore_file, &path_str)?;

    Ok(normalized)
}

fn normalize_path(path: &Path) -> std::io::Result<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::RootDir => {
                normalized.push(component);
            }
            Component::CurDir => continue,
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push(component);
                }
            }
            Component::Normal(c) => {
                normalized.push(c);
            }
            Component::Prefix(p) => {
                normalized.push(p.as_os_str());
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        normalized.push(".");
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
            writeln!(file)?;
        }
        writeln!(file, "{}", line)?;
    }
    Ok(())
}

fn remove_line(file_path: &str, line_to_remove: &str) -> std::io::Result<()> {
    if let Ok(contents) = read_to_string(file_path) {
        let lines: Vec<&str> = contents.lines().filter(|l| *l != line_to_remove).collect();
        if lines.len() < contents.lines().count() {
            let mut file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(file_path)?;

            for line in lines {
                writeln!(file, "{}", line)?;
            }
        }
    }
    Ok(())
}

fn install_hooks() -> std::io::Result<()> {
    let hooks_dir = Path::new(".git/hooks");
    if !hooks_dir.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            ".git directory not found. Are you in a git repository?",
        ));
    }

    let hooks = ["post-checkout", "post-merge"];
    for hook_name in &hooks {
        let hook_path = hooks_dir.join(hook_name);
        
        if !hook_path.exists() {
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .open(&hook_path)?;
            writeln!(file, "#!/bin/sh")?;
        }

        append_if_missing(hook_path.to_str().expect("Valid path"), "shadowbox sync")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(&hook_path)?;
            let mut perms = metadata.permissions();
            perms.set_mode(0o755);
            let _ = std::fs::set_permissions(&hook_path, perms);
        }
    }
    Ok(())
}

fn uninstall_hooks() -> std::io::Result<()> {
    let hooks_dir = Path::new(".git/hooks");
    if !hooks_dir.exists() {
        return Ok(());
    }

    let hooks = ["post-checkout", "post-merge"];
    for hook_name in &hooks {
        let hook_path = hooks_dir.join(hook_name);
        if hook_path.exists() {
            remove_line(hook_path.to_str().expect("Valid path"), "shadowbox sync")?;
        }
    }
    Ok(())
}

fn add_store(name: &str, url: &str) -> std::io::Result<()> {
    let mut config = Config::load()?;
    let store_dir = data_dir()?.join("stores").join(name);
    
    if store_dir.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("Store directory '{}' already exists", store_dir.display()),
        ));
    }

    fs::create_dir_all(&store_dir)?;
    
    // git clone the store
    let status = Command::new("git")
        .args(["clone", url, "."])
        .current_dir(&store_dir)
        .status()?;

    if !status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to clone store repository",
        ));
    }

    config.stores.insert(name.to_string(), StoreConfig { url: url.to_string() });
    config.save()
}

fn list_stores() -> std::io::Result<()> {
    let config = Config::load()?;
    for (name, store) in config.stores {
        println!("{}: {}", name, store.url);
    }
    Ok(())
}

fn add_mapping(pattern: &str, store: &str) -> std::io::Result<()> {
    let mut config = Config::load()?;
    if !config.stores.contains_key(store) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Store '{}' not found", store),
        ));
    }
    config.mappings.insert(pattern.to_string(), store.to_string());
    config.save()
}

fn pull() -> std::io::Result<()> {
    let config = Config::load()?;
    let repo_id = get_repo_id()?;
    let store_name = resolve_store(&config, &repo_id).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "No store mapping found for this repository")
    })?;

    let store_dir = data_dir()?.join("stores").join(store_name);
    
    // git pull store
    Command::new("git")
        .args(["pull"])
        .current_dir(&store_dir)
        .status()?;

    let project_store_path = store_dir.join(&repo_id);
    if !project_store_path.exists() {
        println!("No files in store for this project.");
        return Ok(());
    }

    copy_dir_all(&project_store_path, Path::new("."))?;
    
    // Also sync to gitignore
    sync()?;

    Ok(())
}

fn push() -> std::io::Result<()> {
    let config = Config::load()?;
    let repo_id = get_repo_id()?;
    let store_name = resolve_store(&config, &repo_id).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "No store mapping found for this repository")
    })?;

    let store_dir = data_dir()?.join("stores").join(store_name);
    let project_store_path = store_dir.join(&repo_id);
    
    fs::create_dir_all(&project_store_path)?;

    // Read .shadowbox for files to track
    let shadowbox_file = std::env::var("SHADOWBOX_FILE").unwrap_or(".shadowbox".to_string());
    if let Ok(content) = fs::read_to_string(&shadowbox_file) {
        for line in content.lines() {
            if line.is_empty() { continue; }
            let path = Path::new(line);
            if path.exists() {
                let dest = project_store_path.join(line);
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(path, dest)?;
            }
        }
    }
    
    // Also copy .shadowbox itself
    fs::copy(&shadowbox_file, project_store_path.join(&shadowbox_file))?;

    // Commit and push store
    Command::new("git")
        .args(["add", "."])
        .current_dir(&store_dir)
        .status()?;
        
    Command::new("git")
        .args(["commit", "-m", &format!("Update files for {}", repo_id)])
        .current_dir(&store_dir)
        .status()?;
        
    Command::new("git")
        .args(["push"])
        .current_dir(&store_dir)
        .status()?;

    Ok(())
}

fn get_repo_id() -> std::io::Result<String> {
    // Try to get remote origin URL
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output();
        
    match output {
        Ok(out) if out.status.success() => {
            let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
            // Normalize URL: remove git@, https://, and .git suffix
            let id = url.replace("https://", "")
                        .replace("git@", "")
                        .replace(":", "/")
                        .replace(".git", "");
            Ok(id)
        }
        _ => {
            // Fallback to absolute path
            let path = std::env::current_dir()?;
            Ok(path.to_string_lossy().to_string())
        }
    }
}

fn resolve_store<'a>(config: &'a Config, repo_id: &str) -> Option<&'a String> {
    // Simple exact match first
    if let Some(store) = config.mappings.get(repo_id) {
        return Some(store);
    }
    
    // Try glob matching
    for (pattern, store) in &config.mappings {
        if let Ok(matcher) = glob::Pattern::new(pattern) {
            if matcher.matches(repo_id) {
                return Some(store);
            }
        }
    }
    
    None
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}
