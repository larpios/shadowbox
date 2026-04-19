use crate::config::{Config, data_dir};
use crate::git::{get_repo_id, get_repo_root};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn resolve_store<'a>(config: &'a Config, repo_id: &str) -> Option<&'a String> {
    if let Some(store) = config.mappings.get(repo_id) { return Some(store); }
    for (pattern, store) in &config.mappings {
        if let Ok(matcher) = glob::Pattern::new(pattern) { if matcher.matches(repo_id) { return Some(store); } }
    }
    if config.stores.len() == 1 { return config.stores.keys().next(); }
    None
}

pub fn link(store_name: &str) -> std::io::Result<()> {
    let mut config = Config::load()?;
    let repo_id = get_repo_id()?;
    if !config.stores.contains_key(store_name) { return Err(std::io::Error::new(std::io::ErrorKind::NotFound, format!("Store '{}' not found.", store_name))); }
    config.mappings.insert(repo_id, store_name.to_string());
    config.save()
}

pub fn status() -> std::io::Result<Vec<String>> {
    let config = Config::load()?;
    let repo_id = get_repo_id()?;
    let store_name = resolve_store(&config, &repo_id).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "No mapping"))?;
    let vault_project_path = data_dir()?.join("stores").join(store_name).join(&repo_id);
    let mut results = Vec::new();
    if vault_project_path.exists() {
        for entry in fs::read_dir(&vault_project_path)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name == ".git" { continue; }
            results.push(name);
        }
    }
    Ok(results)
}

pub fn track_file(path_pattern: &str) -> std::io::Result<Vec<PathBuf>> {
    let config = Config::load()?;
    let repo_id = get_repo_id()?;
    let store_name = resolve_store(&config, &repo_id).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Link a vault first"))?;
    let vault_project_path = data_dir()?.join("stores").join(store_name).join(&repo_id);
    let repo_root = get_repo_root()?;
    
    let mut tracked = Vec::new();
    for entry in glob::glob(path_pattern).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))? {
        let local_path = entry.map_err(|e| std::io::Error::other(e.to_string()))?;
        if !local_path.exists() { continue; }
        
        let abs_local = fs::canonicalize(&local_path)?;
        let rel_to_root = abs_local.strip_prefix(&repo_root).map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Outside repo"))?;
        
        let dest = vault_project_path.join(rel_to_root);
        println!("  [TRACK] {} -> vault", rel_to_root.display());
        copy_recursive(&abs_local, &dest)?;
        tracked.push(rel_to_root.to_path_buf());
    }
    Ok(tracked)
}

pub fn untrack_file(path: &str) -> std::io::Result<PathBuf> {
    let repo_root = get_repo_root()?;
    let abs_path = fs::canonicalize(Path::new(path))?;
    let rel_to_root = abs_path.strip_prefix(&repo_root).map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Outside repo"))?;
    let config = Config::load()?;
    let repo_id = get_repo_id()?;
    let store_name = resolve_store(&config, &repo_id).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "No mapping"))?;
    let target = data_dir()?.join("stores").join(store_name).join(&repo_id).join(rel_to_root);
    if target.exists() {
        if target.is_dir() { fs::remove_dir_all(target)?; } else { fs::remove_file(target)?; }
    }
    Ok(rel_to_root.to_path_buf())
}

pub fn push() -> std::io::Result<()> {
    let config = Config::load()?;
    let repo_id = get_repo_id()?;
    let store_name = resolve_store(&config, &repo_id).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "No mapping"))?;
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
        if name == ".git" { continue; }
        
        let local_path = repo_root.join(&name);
        let vault_path = vault_project_path.join(&name);
        
        if local_path.exists() {
            println!("  [SYNC] {}", name.to_string_lossy());
            copy_recursive(&local_path, &vault_path)?;
        } else {
            println!("  [DELETE] {} (not found locally)", name.to_string_lossy());
            if vault_path.is_dir() { fs::remove_dir_all(&vault_path)?; } else { fs::remove_file(&vault_path)?; }
        }
    }

    // FORCE add to bypass global/local gitignores
    Command::new("git").current_dir(&store_dir).args(["add", "--all", "--force", "."]).status()?;
    
    let commit = Command::new("git").current_dir(&store_dir)
        .args(["commit", "-m", &format!("Update {}", repo_id), "--no-verify"])
        .status()?;
    
    Command::new("git").current_dir(&store_dir).args(["push", "--no-verify"]).status()?;

    if commit.success() { println!("Vault updated."); }
    else { println!("Vault up-to-date."); }
    Ok(())
}

pub fn pull() -> std::io::Result<()> {
    let config = Config::load()?;
    let repo_id = get_repo_id()?;
    let store_name = resolve_store(&config, &repo_id).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "No mapping"))?;
    let store_dir = data_dir()?.join("stores").join(store_name);
    let vault_project_path = store_dir.join(&repo_id);
    let repo_root = get_repo_root()?;

    println!("Pulling from vault...");
    Command::new("git").current_dir(&store_dir).args(["pull", "--no-verify"]).status()?;
    
    if vault_project_path.exists() {
        println!("Restoring tracked files...");
        copy_recursive(&vault_project_path, &repo_root)?;
    }
    Ok(())
}

pub fn copy_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if src.is_dir() {
        if !dst.exists() { fs::create_dir_all(dst)?; }
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let name = entry.file_name();
            if name == ".git" { continue; }
            copy_recursive(&entry.path(), &dst.join(name))?;
        }
    } else {
        if let Some(parent) = dst.parent() { if !parent.exists() { fs::create_dir_all(parent)?; } }
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
    if let Some(s) = resolve_store(&config, &repo_id) { println!("Active Vault: {}", s); }
    Ok(())
}
