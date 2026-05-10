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

pub fn sync_recursive(src: &Path, dst: &Path, follow_links: bool) -> std::io::Result<()> {
    let src_metadata = fs::symlink_metadata(src)?;

    if src_metadata.is_symlink() && !follow_links {
        let target = fs::read_link(src)?;

        if let Some(parent) = dst.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent)?;
        }

        if fs::symlink_metadata(dst).is_ok() {
            if dst.is_dir() && !fs::symlink_metadata(dst)?.is_symlink() {
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
        return Ok(());
    }

    let metadata = if follow_links {
        fs::metadata(src)?
    } else {
        src_metadata
    };

    if metadata.is_dir() {
        if fs::symlink_metadata(dst).is_ok()
            && (!dst.is_dir() || fs::symlink_metadata(dst)?.is_symlink())
        {
            fs::remove_file(dst)?;
        }
        if !dst.exists() {
            fs::create_dir_all(dst)?;
        }

        let mut src_names: std::collections::HashSet<std::ffi::OsString> =
            std::collections::HashSet::new();
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let name = entry.file_name();
            if name == ".git" {
                continue;
            }
            src_names.insert(name.clone());
            sync_recursive(&entry.path(), &dst.join(name), follow_links)?;
        }

        for entry in fs::read_dir(dst)? {
            let entry = entry?;
            let name = entry.file_name();
            if name == ".git" {
                continue;
            }
            if !src_names.contains(&name) {
                let path = entry.path();
                let meta = fs::symlink_metadata(&path)?;
                if meta.is_dir() && !meta.is_symlink() {
                    fs::remove_dir_all(&path)?;
                } else {
                    fs::remove_file(&path)?;
                }
            }
        }
    } else {
        if let Some(parent) = dst.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent)?;
        }
        if fs::symlink_metadata(dst).is_ok()
            && dst.is_dir()
            && !fs::symlink_metadata(dst)?.is_symlink()
        {
            fs::remove_dir_all(dst)?;
        }
        fs::copy(src, dst)?;
    }
    Ok(())
}

pub fn copy_recursive(src: &Path, dst: &Path, follow_links: bool) -> std::io::Result<()> {
    let src_metadata = fs::symlink_metadata(src)?;

    if src_metadata.is_symlink() && !follow_links {
        let target = fs::read_link(src)?;

        if let Some(parent) = dst.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent)?;
        }

        // Check if destination exists (even if it's a broken symlink)
        if fs::symlink_metadata(dst).is_ok() {
            if dst.is_dir() && !fs::symlink_metadata(dst)?.is_symlink() {
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
        return Ok(());
    }

    // If we are following links, get the metadata of the target
    let metadata = if follow_links {
        fs::metadata(src)?
    } else {
        src_metadata
    };

    if metadata.is_dir() {
        if fs::symlink_metadata(dst).is_ok()
            && (!dst.is_dir() || fs::symlink_metadata(dst)?.is_symlink())
        {
            fs::remove_file(dst)?;
        }
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
        // If destination is a directory but source is a file, remove the directory
        if fs::symlink_metadata(dst).is_ok()
            && dst.is_dir()
            && !fs::symlink_metadata(dst)?.is_symlink()
        {
            fs::remove_dir_all(dst)?;
        }
        // fs::copy will overwrite files/symlinks
        fs::copy(src, dst)?;
    }
    Ok(())
}

pub fn is_binary(path: &Path) -> std::io::Result<bool> {
    use std::io::Read;
    let mut file = fs::File::open(path)?;
    let mut buffer = [0u8; 1024];
    let n = file.read(&mut buffer)?;
    Ok(buffer[..n].contains(&0))
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
