use std::path::{Path, PathBuf};
use std::process::Command;

pub fn get_repo_root() -> std::io::Result<PathBuf> {
    let output = Command::new("git").args(["rev-parse", "--show-toplevel"]).output()?;
    if output.status.success() {
        let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(PathBuf::from(path_str))
    } else {
        std::env::current_dir()
    }
}

pub fn get_repo_id() -> std::io::Result<String> {
    let output = Command::new("git").args(["remote", "get-url", "origin"]).output();
    match output {
        Ok(out) if out.status.success() => {
            let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
            Ok(url.replace("https://", "").replace("git@", "").replace(":", "/").replace(".git", ""))
        }
        _ => {
            let path = std::env::current_dir()?;
            Ok(path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "default".to_string()))
        }
    }
}

pub fn install_hooks() -> std::io::Result<()> { Ok(()) }
pub fn uninstall_hooks() -> std::io::Result<()> { Ok(()) }
