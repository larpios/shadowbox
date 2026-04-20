use crate::config::Config;
use crate::git::{get_repo_id, get_repo_root};
use crate::utils::get_vault_project_path;
use std::fs;
use std::path::Path;

pub fn run() -> std::io::Result<()> {
    let config = Config::load()?;
    let repo_id = get_repo_id()?;
    let vault_project_path = get_vault_project_path(&config, &repo_id)?;
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

    for file in results {
        println!("{}", file);
    }
    Ok(())
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
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.is_dir() {
            collect_status(vault_root, &path, repo_root, results)?;
        } else {
            let rel_path = path.strip_prefix(vault_root).unwrap();
            let local_path = repo_root.join(rel_path);
            let display_name = rel_path.to_string_lossy().to_string();

            // Use symlink_metadata to correctly check for existence of symlinks (even broken ones)
            if fs::symlink_metadata(&local_path).is_ok() {
                results.push(display_name);
            } else {
                results.push(format!("[MISSING] {}", display_name));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use tempfile::tempdir;

    fn run_shadowbox(args: Vec<&str>, current_dir: &Path, home_dir: &Path) -> std::process::Output {
        let binary_path = env::current_dir().unwrap().join("target/debug/shadowbox");
        Command::new(&binary_path)
            .args(args)
            .current_dir(current_dir)
            .env("HOME", home_dir)
            .env("XDG_CONFIG_HOME", home_dir.join(".config"))
            .env("XDG_DATA_HOME", home_dir.join(".local/share"))
            .output()
            .expect("Failed to execute shadowbox")
    }

    #[test]
    fn test_status_command() {
        let home_dir = tempdir().expect("Failed to create temp home");
        let remote_store_dir = tempdir().expect("Failed to create temp remote store");
        let dir = tempdir().expect("Failed to create temp project");

        Command::new("git")
            .args(["init", "--bare"])
            .current_dir(remote_store_dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["remote", "add", "origin", "https://github.com/user/project"])
            .current_dir(dir.path())
            .status()
            .unwrap();

        run_shadowbox(
            vec![
                "store",
                "add",
                "my_store",
                &remote_store_dir.path().to_string_lossy(),
            ],
            dir.path(),
            home_dir.path(),
        );
        run_shadowbox(vec!["map", "**", "my_store"], dir.path(), home_dir.path());

        let file1 = "file1.txt";
        fs::write(dir.path().join(file1), "content").unwrap();
        run_shadowbox(vec!["track", file1], dir.path(), home_dir.path());

        let output = run_shadowbox(vec!["status"], dir.path(), home_dir.path());
        assert!(String::from_utf8_lossy(&output.stdout).contains(file1));
    }

    #[test]
    fn test_status_missing_file() {
        let home_dir = tempdir().expect("Failed to create temp home");
        let remote_store_dir = tempdir().expect("Failed to create temp remote store");
        let dir = tempdir().expect("Failed to create temp project");

        Command::new("git")
            .args(["init", "--bare"])
            .current_dir(remote_store_dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["remote", "add", "origin", "https://github.com/user/project"])
            .current_dir(dir.path())
            .status()
            .unwrap();

        run_shadowbox(
            vec![
                "store",
                "add",
                "my_store",
                &remote_store_dir.path().to_string_lossy(),
            ],
            dir.path(),
            home_dir.path(),
        );
        run_shadowbox(vec!["map", "**", "my_store"], dir.path(), home_dir.path());

        let file1 = "missing.txt";
        fs::write(dir.path().join(file1), "content").unwrap();
        run_shadowbox(vec!["track", file1], dir.path(), home_dir.path());
        fs::remove_file(dir.path().join(file1)).unwrap();

        let output = run_shadowbox(vec!["status"], dir.path(), home_dir.path());
        assert!(String::from_utf8_lossy(&output.stdout).contains("[MISSING] missing.txt"));
    }
}
