use crate::config::{Config, data_dir};
use crate::git::get_repo_id;
use std::fs;
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

pub fn link(store_name: &str) -> std::io::Result<()> {
    let mut config = Config::load()?;
    let repo_id = get_repo_id()?;
    
    if !config.stores.contains_key(store_name) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Store '{}' not found. Use 'shadowbox store add' first.", store_name),
        ));
    }

    config.mappings.insert(repo_id, store_name.to_string());
    config.save()
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
    let project_store_path = store_dir.join(&repo_id);

    let mut results = Vec::new();
    if project_store_path.exists() {
        collect_managed_files(&project_store_path, Path::new(""), &mut results)?;
    }
    Ok(results)
}

fn collect_managed_files(base: &Path, relative: &Path, results: &mut Vec<String>) -> std::io::Result<()> {
    for entry in fs::read_dir(base.join(relative))? {
        let entry = entry?;
        let name = entry.file_name();
        if name == ".git" || name == ".shadowbox" { continue; }
        
        let rel_path = relative.join(name);
        let local_path = Path::new(".").join(&rel_path);
        
        if entry.file_type()?.is_dir() {
            collect_managed_files(base, &rel_path, results)?;
        } else {
            if local_path.exists() {
                results.push(rel_path.to_string_lossy().to_string());
            } else {
                results.push(format!("[MISSING] {}", rel_path.to_string_lossy()));
            }
        }
    }
    Ok(())
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
            "No store mapping found for this repository. Use 'shadowbox link' or 'shadowbox map' first.",
        )
    })?;
    let store_dir = data_dir()?.join("stores").join(store_name);
    let project_store_path = store_dir.join(&repo_id);
    fs::create_dir_all(&project_store_path)?;

    for entry in entries {
        let path =
            entry.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        if !path.exists() {
            continue;
        }

        let normalized = normalize_path(&path)?;
        let dest = project_store_path.join(&normalized);
        
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        if path.is_dir() {
            copy_dir_all(&path, &dest)?;
        } else {
            fs::copy(&path, &dest)?;
        }
        
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
    let target_in_store = project_store_path.join(&normalized);

    if target_in_store.exists() {
        if target_in_store.is_dir() {
            fs::remove_dir_all(target_in_store)?;
        } else {
            fs::remove_file(target_in_store)?;
        }
    }

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

    // Copy everything from store to project
    copy_dir_all(&project_store_path, Path::new("."))?;

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

    if !project_store_path.exists() {
        println!("No files are being managed for this project. Use 'shadowbox track' first.");
        return Ok(());
    }

    // Update store with local changes of managed files
    update_store_from_local(&project_store_path, Path::new(""))?;

    // LFS Support
    if let Ok(lfs_check) = Command::new("git-lfs").arg("version").output() {
        if lfs_check.status.success() {
            // Ensure LFS is initialized in the store
            let _ = Command::new("git-lfs")
                .args(["install", "--local"])
                .current_dir(&store_dir)
                .status();

            // Track large files in the store
            track_large_files_in_lfs(&store_dir, &project_store_path, Path::new(""), &repo_id)?;
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

fn update_store_from_local(project_store_path: &Path, relative: &Path) -> std::io::Result<()> {
    let current_dir_in_store = project_store_path.join(relative);
    for entry in fs::read_dir(current_dir_in_store)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == ".git" || name == ".shadowbox" { continue; }
        
        let rel_path = relative.join(name);
        let local_path = Path::new(".").join(&rel_path);
        let store_path = project_store_path.join(&rel_path);
        
        if entry.file_type()?.is_dir() {
            update_store_from_local(project_store_path, &rel_path)?;
        } else {
            if local_path.exists() {
                fs::copy(&local_path, &store_path)?;
            }
        }
    }
    Ok(())
}

fn track_large_files_in_lfs(store_dir: &Path, project_store_path: &Path, relative: &Path, repo_id: &str) -> std::io::Result<()> {
    let current_dir_in_store = project_store_path.join(relative);
    for entry in fs::read_dir(current_dir_in_store)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == ".git" || name == ".shadowbox" { continue; }
        
        let rel_path = relative.join(name);
        let store_path = project_store_path.join(&rel_path);
        
        if entry.file_type()?.is_dir() {
            track_large_files_in_lfs(store_dir, project_store_path, &rel_path, repo_id)?;
        } else {
            if let Ok(metadata) = fs::metadata(&store_path) {
                if metadata.len() > 5 * 1024 * 1024 {
                    // 5MB threshold
                    let lfs_path = Path::new(repo_id).join(&rel_path);
                    let _ = Command::new("git-lfs")
                        .args(["track", &lfs_path.to_string_lossy()])
                        .current_dir(store_dir)
                        .status();
                }
            }
        }
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
    if src.is_dir() {
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
    } else {
        fs::copy(src, dst)?;
    }
    Ok(())
}
