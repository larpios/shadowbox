use std::fs;
use std::process::Command;
use crate::config::{Config, StoreConfig, data_dir};

pub fn add_store(name: &str, url: &str) -> std::io::Result<()> {
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
    config.mappings.insert(pattern.to_string(), store.to_string());
    config.save()
}

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
    
    // 3. Fallback: If exactly one store exists, use it as default
    if config.stores.len() == 1 {
        return config.stores.keys().next();
    }
    
    None
}
