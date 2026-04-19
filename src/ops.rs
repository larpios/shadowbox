use crate::config::{Config, data_dir};
use crate::git::{append_if_missing, get_repo_id, remove_line};
use std::fs::{self, OpenOptions, read_to_string};
use std::path::{Component, Path, PathBuf};
use std::process::Command;

pub fn resolve_store<'a>(config: &'a Config, repo_id: &str) -> Option<&'a String> {
    // 1. Simple exact match first
    if let Some(store) = config.mappings.get(repo_id) {
        return Some(store);
    }

    // 2. Try glob matching
    for (pattern, store) in &config.mappings {
        if let Ok(matcher) = glob::Pattern::new(pattern) {
            if matcher.matches(repo_id) {
                return Some(store);
            }
        }
    }

    // 3. Fallback: ONLY if exactly one store exists total
    if config.stores.len() == 1 {
        return config.stores.keys().next();
    }

    // 4. Ambiguity: multiple stores but no mapping
    None
}

pub fn init() -> std::io::Result<()> {
    let gitignore_file = std::env::var("GITIGNORE_FILE").unwrap_or(".gitignore".to_string());

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

pub fn sync() -> std::io::Result<()> {
    let config = Config::load()?;
    let repo_id = get_repo_id()?;
    let store_name = resolve_store(&config, &repo_id).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No store mapping found for this repository",
        )
    })?;
    let store_dir = data_dir()?.join("stores").join(store_name);
    let shadowbox_file = store_dir.join(&repo_id).join(".shadowbox");

    let gitignore_file = std::env::var("GITIGNORE_FILE").unwrap_or(".gitignore".to_string());

    if shadowbox_file.exists() {
        let contents = read_to_string(&shadowbox_file).unwrap_or_default();
        for line in contents.lines() {
            if line.is_empty() {
                continue;
            }
            append_if_missing(&gitignore_file, line)?;
        }
    }
    Ok(())
}

pub fn status() -> std::io::Result<Vec<String>> {
    let config = Config::load()?;
    let repo_id = get_repo_id()?;
    let store_name = resolve_store(&config, &repo_id).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No store mapping found for this repository",
        )
    })?;
    let store_dir = data_dir()?.join("stores").join(store_name);
    let shadowbox_file = store_dir.join(&repo_id).join(".shadowbox");

    let mut results = Vec::new();
    if shadowbox_file.exists() {
        let contents = read_to_string(shadowbox_file).unwrap_or_default();
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
    }
    Ok(results)
}

pub fn track_file(path_pattern: &str) -> std::io::Result<Vec<PathBuf>> {
    let mut tracked_paths = Vec::new();

    // Support globs
    let entries = glob::glob(path_pattern)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?;

    let config = Config::load()?;
    let repo_id = get_repo_id()?;
    let store_name = resolve_store(&config, &repo_id).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No store mapping found for this repository. Use 'shadowbox map' first.",
        )
    })?;
    let store_dir = data_dir()?.join("stores").join(store_name);
    let project_store_path = store_dir.join(&repo_id);
    fs::create_dir_all(&project_store_path)?;
    let shadowbox_file = project_store_path.join(".shadowbox");

    let gitignore_file = std::env::var("GITIGNORE_FILE").unwrap_or(".gitignore".to_string());

    for entry in entries {
        let path =
            entry.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        if !path.exists() {
            continue;
        }

        let normalized = normalize_path(&path)?;
        let path_str = normalized.to_string_lossy();

        append_if_missing(shadowbox_file.to_str().unwrap(), &path_str)?;
        append_if_missing(&gitignore_file, &path_str)?;
        tracked_paths.push(normalized);
    }

    if tracked_paths.is_empty() && !path_pattern.contains('*') {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("File '{}' not found", path_pattern),
        ));
    }

    Ok(tracked_paths)
}

pub fn untrack_file(path: &str) -> std::io::Result<PathBuf> {
    let raw_path = Path::new(path);
    let normalized = normalize_path(raw_path)?;
    let path_str = normalized.to_string_lossy();

    let config = Config::load()?;
    let repo_id = get_repo_id()?;
    let store_name = resolve_store(&config, &repo_id).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No store mapping found for this repository",
        )
    })?;
    let store_dir = data_dir()?.join("stores").join(store_name);
    let shadowbox_file = store_dir.join(&repo_id).join(".shadowbox");

    let gitignore_file = std::env::var("GITIGNORE_FILE").unwrap_or(".gitignore".to_string());

    if shadowbox_file.exists() {
        remove_line(shadowbox_file.to_str().unwrap(), &path_str)?;
    }
    remove_line(&gitignore_file, &path_str)?;

    Ok(normalized)
}

pub fn pull() -> std::io::Result<()> {
    let config = Config::load()?;
    let repo_id = get_repo_id()?;
    let store_name = resolve_store(&config, &repo_id).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No store mapping found for this repository",
        )
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

    // Copy everything except .shadowbox
    for entry in fs::read_dir(&project_store_path)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == ".shadowbox" {
            continue;
        }
        let dest = Path::new(".").join(&name);
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dest)?;
        } else {
            fs::copy(entry.path(), dest)?;
        }
    }

    // Also sync to gitignore
    sync()?;

    Ok(())
}

pub fn push() -> std::io::Result<()> {
    let config = Config::load()?;
    let repo_id = get_repo_id()?;
    let store_name = resolve_store(&config, &repo_id).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No store mapping found for this repository",
        )
    })?;

    let store_dir = data_dir()?.join("stores").join(store_name);
    let project_store_path = store_dir.join(&repo_id);
    let shadowbox_file = project_store_path.join(".shadowbox");

    if !shadowbox_file.exists() {
        println!("No files are being tracked for this project. Use 'shadowbox track' first.");
        return Ok(());
    }

    let shadowbox_content = fs::read_to_string(&shadowbox_file)?;

    // Cleanup existing files in the store for this project (except .shadowbox)
    // We'll just read into memory, wipe, and restore .shadowbox
    if project_store_path.exists() {
        fs::remove_dir_all(&project_store_path)?;
    }
    fs::create_dir_all(&project_store_path)?;
    fs::write(&shadowbox_file, &shadowbox_content)?;

    // Read tracked files from the memory-cached content
    for line in shadowbox_content.lines() {
        if line.is_empty() {
            continue;
        }
        let path = Path::new(line);
        if path.exists() {
            let dest = project_store_path.join(line);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            if path.is_dir() {
                copy_dir_all(path, &dest)?;
            } else {
                fs::copy(path, dest)?;
            }
        }
    }

    // LFS Support
    if let Ok(lfs_check) = Command::new("git-lfs").arg("version").output() {
        if lfs_check.status.success() {
            // Ensure LFS is initialized in the store
            let _ = Command::new("git-lfs")
                .args(["install", "--local"])
                .current_dir(&store_dir)
                .status();

            // Track files over 5MB with LFS
            for line in shadowbox_content.lines() {
                if line.is_empty() {
                    continue;
                }
                let path = Path::new(line);
                if let Ok(metadata) = fs::metadata(path) {
                    if metadata.len() > 5 * 1024 * 1024 {
                        // 5MB threshold
                        let lfs_path = Path::new(&repo_id).join(line);
                        Command::new("git-lfs")
                            .args(["track", &lfs_path.to_string_lossy()])
                            .current_dir(&store_dir)
                            .status()?;
                    }
                }
            }
        }
    }

    // Commit and push store
    Command::new("git")
        .args(["add", "."])
        .current_dir(&store_dir)
        .status()?;

    // Check if there are changes to commit
    let status = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .current_dir(&store_dir)
        .status()?;

    if !status.success() {
        Command::new("git")
            .args(["commit", "-m", &format!("Update files for {}", repo_id)])
            .current_dir(&store_dir)
            .status()?;

        Command::new("git")
            .args(["push"])
            .current_dir(&store_dir)
            .status()?;
    } else {
        println!("Nothing to push for {}.", repo_id);
    }

    Ok(())
}

pub fn normalize_path(path: &Path) -> std::io::Result<PathBuf> {
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

pub fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
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
