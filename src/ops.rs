use crate::config::{Config, data_dir};
use crate::git::{get_repo_id, get_repo_root};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn resolve_store<'a>(config: &'a Config, repo_id: &str) -> Option<&'a String> {
    if let Some(store) = config.mappings.get(repo_id) {
        return Some(store);
    }
    for (pattern, store) in &config.mappings {
        if let Ok(matcher) = glob::Pattern::new(pattern)
            && matcher.matches(repo_id)
        {
            return Some(store);
        }
    }
    if config.stores.len() == 1 {
        return config.stores.keys().next();
    }
    None
}

pub fn link(store_name: &str) -> std::io::Result<()> {
    let mut config = Config::load()?;
    let repo_id = get_repo_id()?;
    if !config.stores.contains_key(store_name) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Store '{}' not found.", store_name),
        ));
    }
    config.mappings.insert(repo_id, store_name.to_string());
    config.save()
}

pub fn status() -> std::io::Result<Vec<String>> {
    let config = Config::load()?;
    let repo_id = get_repo_id()?;
    let store_name = resolve_store(&config, &repo_id)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "No mapping"))?;
    let vault_project_path = data_dir()?.join("stores").join(store_name).join(&repo_id);
    let repo_root = get_repo_root()?;
    let mut results = Vec::new();
    if vault_project_path.exists() {
        collect_status(
            &vault_project_path,
            &vault_project_path,
            &repo_root,
            &mut results,
        )?;
    }
    Ok(results)
}

fn collect_status(
    vault_root: &Path,
    current_dir: &Path,
    repo_root: &Path,
    results: &mut Vec<String>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        if name == ".git" {
            continue;
        }
        if path.is_dir() {
            collect_status(vault_root, &path, repo_root, results)?;
        } else {
            let rel_path = path.strip_prefix(vault_root).unwrap();
            let local_path = repo_root.join(rel_path);
            let display_name = rel_path.to_string_lossy().to_string();
            if local_path.exists() {
                results.push(display_name);
            } else {
                results.push(format!("[MISSING] {}", display_name));
            }
        }
    }
    Ok(())
}

pub fn track_files(path_patterns: &[String], follow_links: bool) -> std::io::Result<Vec<PathBuf>> {
    let mut tracked = Vec::new();
    for pattern in path_patterns {
        tracked.append(&mut track_file(pattern, follow_links)?);
    }
    Ok(tracked)
}

fn track_file(path_pattern: &str, follow_links: bool) -> std::io::Result<Vec<PathBuf>> {
    let config = Config::load()?;
    let repo_id = get_repo_id()?;
    let store_name = resolve_store(&config, &repo_id)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Link a vault first"))?;
    let store_dir = data_dir()?.join("stores").join(store_name);
    let vault_project_path = store_dir.join(&repo_id);
    let repo_root = get_repo_root()?;

    let mut tracked = Vec::new();
    for entry in glob::glob(path_pattern)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?
    {
        let local_path = entry.map_err(|e| std::io::Error::other(e.to_string()))?;
        let _ = fs::symlink_metadata(&local_path)?;

        // For destination path in vault, we always use the path as specified (relative to repo root)
        let abs_for_rel = std::path::absolute(&local_path)?;

        // For source path to copy from, we follow links only if follow_links is true
        let src_path = if follow_links {
            fs::canonicalize(&local_path)?
        } else {
            abs_for_rel.clone()
        };

        let rel_to_root = abs_for_rel
            .strip_prefix(&repo_root)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Outside repo"))?;

        let dest = vault_project_path.join(rel_to_root);
        println!("  [TRACK] {} -> vault", rel_to_root.display());
        copy_recursive(&src_path, &dest, follow_links)?;
        // LFS support
        if is_lfs_available() {
            let metadata = fs::metadata(&src_path)?;
            if metadata.is_file() && metadata.len() > 5 * 1024 * 1024 {
                // Only track in LFS if it's not a symlink OR we followed it
                let is_symlink = fs::symlink_metadata(&src_path)?.is_symlink();
                if !is_symlink || follow_links {
                    // Track in LFS in the store
                    let rel_in_store = PathBuf::from(&repo_id).join(rel_to_root);
                    let _ = Command::new("git-lfs")
                        .current_dir(&store_dir)
                        .args(["track", &rel_in_store.to_string_lossy()])
                        .status();
                }
            }
        }

        tracked.push(rel_to_root.to_path_buf());
    }
    Ok(tracked)
}

fn is_lfs_available() -> bool {
    Command::new("git-lfs")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn untrack_file(path: &str) -> std::io::Result<PathBuf> {
    let repo_root = get_repo_root()?;
    let abs_path = std::path::absolute(Path::new(path))?;
    let rel_to_root = abs_path
        .strip_prefix(&repo_root)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Outside repo"))?;
    let config = Config::load()?;
    let repo_id = get_repo_id()?;
    let store_name = resolve_store(&config, &repo_id)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "No mapping"))?;
    let target = data_dir()?
        .join("stores")
        .join(store_name)
        .join(&repo_id)
        .join(rel_to_root);
    if target.exists() {
        if target.is_dir() {
            fs::remove_dir_all(target)?;
        } else {
            fs::remove_file(target)?;
        }
    }
    Ok(rel_to_root.to_path_buf())
}

pub fn push() -> std::io::Result<()> {
    let config = Config::load()?;
    let repo_id = get_repo_id()?;
    let store_name = resolve_store(&config, &repo_id)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "No mapping"))?;
    let store_dir = data_dir()?.join("stores").join(store_name);
    let vault_project_path = store_dir.join(&repo_id);
    let repo_root = get_repo_root()?;

    if !vault_project_path.exists() {
        println!("Nothing tracked for this project.");
        return Ok(());
    }

    println!("Syncing updates for {}...", repo_id);
    for entry in fs::read_dir(&vault_project_path)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == ".git" {
            continue;
        }

        let local_path = repo_root.join(&name);
        let vault_path = vault_project_path.join(&name);

        if fs::symlink_metadata(&local_path).is_ok() {
            println!("  [SYNC] {}", name.to_string_lossy());
            copy_recursive(&local_path, &vault_path, false)?;
        } else {
            println!("  [DELETE] {} (not found locally)", name.to_string_lossy());
            if let Ok(meta) = fs::symlink_metadata(&vault_path) {
                if meta.is_dir() {
                    fs::remove_dir_all(&vault_path)?;
                } else {
                    fs::remove_file(&vault_path)?;
                }
            }
        }
    }

    // FORCE add to bypass global/local gitignores
    Command::new("git")
        .current_dir(&store_dir)
        .args(["add", "--all", "--force", "."])
        .status()?;

    let commit = Command::new("git")
        .current_dir(&store_dir)
        .args([
            "commit",
            "-m",
            &format!("Update {}", repo_id),
            "--no-verify",
        ])
        .status()?;

    Command::new("git")
        .current_dir(&store_dir)
        .args(["push", "--no-verify"])
        .status()?;

    if commit.success() {
        println!("Vault updated.");
    } else {
        println!("Vault up-to-date.");
    }
    Ok(())
}

pub fn pull() -> std::io::Result<()> {
    let config = Config::load()?;
    let repo_id = get_repo_id()?;
    let store_name = resolve_store(&config, &repo_id)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "No mapping"))?;
    let store_dir = data_dir()?.join("stores").join(store_name);
    let vault_project_path = store_dir.join(&repo_id);
    let repo_root = get_repo_root()?;

    println!("Pulling from vault...");
    Command::new("git")
        .current_dir(&store_dir)
        .args(["pull", "--no-verify"])
        .status()?;

    if vault_project_path.exists() {
        println!("Restoring tracked files...");
        copy_recursive(&vault_project_path, &repo_root, false)?;
    }
    Ok(())
}

pub fn copy_recursive(src: &Path, dst: &Path, follow_links: bool) -> std::io::Result<()> {
    let metadata = fs::symlink_metadata(src)?;

    if metadata.is_symlink() && !follow_links {
        let target = fs::read_link(src)?;
        if let Some(parent) = dst.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent)?;
        }
        if dst.exists() {
            if dst.is_dir() {
                fs::remove_dir_all(dst)?;
            } else {
                fs::remove_file(dst)?;
            }
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(target, dst)?;
        #[cfg(windows)]
        {
            if let Ok(target_meta) = fs::metadata(src) {
                if target_meta.is_dir() {
                    std::os::windows::fs::symlink_dir(target, dst)?;
                } else {
                    std::os::windows::fs::symlink_file(target, dst)?;
                }
            } else {
                std::os::windows::fs::symlink_file(target, dst)?;
            }
        }
    } else if metadata.is_dir() {
        if !dst.exists() {
            fs::create_dir_all(dst)?;
        }
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let name = entry.file_name();
            if name == ".git" {
                continue;
            }
            copy_recursive(&entry.path(), &dst.join(name), follow_links)?;
        }
    } else {
        if let Some(parent) = dst.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst)?;
    }
    Ok(())
}

pub fn debug_info() -> std::io::Result<()> {
    let config = Config::load()?;
    let repo_id = get_repo_id()?;
    println!("--- Shadowbox Diagnostic ---");
    println!("Version:     {}", env!("CARGO_PKG_VERSION"));
    println!("Repo ID:     {}", repo_id);
    println!("Vault Dir:   {}", data_dir()?.join("stores").display());
    if let Some(s) = resolve_store(&config, &repo_id) {
        println!("Active Vault: {}", s);
    }
    Ok(())
}
