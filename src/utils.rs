use crate::config::{Config, data_dir};
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

pub fn is_lfs_available() -> bool {
    Command::new("git-lfs")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
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

pub fn get_vault_project_path(config: &Config, repo_id: &str) -> std::io::Result<PathBuf> {
    let store_name = resolve_store(config, repo_id)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Link a vault first"))?;
    Ok(data_dir()?.join("stores").join(store_name).join(repo_id))
}

pub fn get_store_dir(config: &Config, repo_id: &str) -> std::io::Result<PathBuf> {
    let store_name = resolve_store(config, repo_id)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Link a vault first"))?;
    Ok(data_dir()?.join("stores").join(store_name))
}
