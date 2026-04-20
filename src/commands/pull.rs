use crate::config::Config;
use crate::git::{get_repo_id, get_repo_root};
use crate::utils::{copy_recursive, get_store_dir, get_vault_project_path};
use std::process::Command;

pub fn run() -> std::io::Result<()> {
    let config = Config::load()?;
    let repo_id = get_repo_id()?;
    let vault_project_path = get_vault_project_path(&config, &repo_id)?;
    let store_dir = get_store_dir(&config, &repo_id)?;
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
    println!("Pull complete.");
    Ok(())
}
