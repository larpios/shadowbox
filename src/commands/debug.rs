use crate::config::{Config, data_dir};
use crate::git::get_repo_id;
use crate::utils::resolve_store;

pub fn run() -> std::io::Result<()> {
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
