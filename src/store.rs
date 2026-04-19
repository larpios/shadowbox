use crate::config::{Config, StoreConfig, data_dir};
use std::fs;
use std::process::Command;

pub fn add_store(name: &str, url: &str, force: bool) -> std::io::Result<()> {
    let mut config = Config::load()?;
    let store_dir = data_dir()?.join("stores").join(name);

    if store_dir.exists() {
        if force {
            fs::remove_dir_all(&store_dir)?;
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!(
                    "Store directory '{}' already exists. Use --force to overwrite it.",
                    store_dir.display()
                ),
            ));
        }
    }

    fs::create_dir_all(&store_dir)?;

    // Auto-protocol: Prepend https:// if missing, looks like a URL, and NOT a local path
    let git_url = if !url.contains("://") && !url.contains('@') && !url.starts_with('/') {
        format!("https://{}", url)
    } else {
        url.to_string()
    };

    // git clone the store
    let status = Command::new("git")
        .args(["clone", &git_url, "."])
        .current_dir(&store_dir)
        .status()?;

    if !status.success() {
        // CLEANUP: delete the directory if clone failed so user can retry
        let _ = fs::remove_dir_all(&store_dir);
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to clone store repository. Check your URL/permissions and try again.",
        ));
    }

    config
        .stores
        .insert(name.to_string(), StoreConfig { url: git_url });
    config.save()
}

pub fn list_stores() -> std::io::Result<()> {
    let config = Config::load()?;
    for (name, store) in config.stores {
        println!("{}: {}", name, store.url);
    }
    Ok(())
}

pub fn add_mapping(pattern: &str, store: &str) -> std::io::Result<()> {
    let mut config = Config::load()?;
    if !config.stores.contains_key(store) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Store '{}' not found", store),
        ));
    }
    config
        .mappings
        .insert(pattern.to_string(), store.to_string());
    config.save()
}
