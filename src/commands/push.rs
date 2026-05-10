use crate::config::Config;
use crate::git::{get_repo_id, get_repo_root};
use crate::utils::{get_store_dir, get_vault_project_path, sync_recursive};
use std::fs;
use std::process::Command;

pub fn run() -> std::io::Result<()> {
    let config = Config::load()?;
    let repo_id = get_repo_id()?;
    let vault_project_path = get_vault_project_path(&config, &repo_id)?;
    let store_dir = get_store_dir(&config, &repo_id)?;
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
            sync_recursive(&local_path, &vault_path, false)?;
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
    println!("Push complete.");
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
    fn test_distributed_push_pull() {
        let home_dir = tempdir().expect("Failed to create temp home");
        let remote_store_dir = tempdir().expect("Failed to create temp remote store");
        let project_remote_dir = tempdir().expect("Failed to create temp project remote");
        let project_dir_a = tempdir().expect("Failed to create temp project A");
        let project_dir_b = tempdir().expect("Failed to create temp project B");

        Command::new("git")
            .args(["init", "--bare"])
            .current_dir(remote_store_dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["init", "--bare"])
            .current_dir(project_remote_dir.path())
            .status()
            .unwrap();

        Command::new("git")
            .args(["init"])
            .current_dir(project_dir_a.path())
            .status()
            .unwrap();
        Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                &project_remote_dir.path().to_string_lossy(),
            ])
            .current_dir(project_dir_a.path())
            .status()
            .unwrap();

        run_shadowbox(
            vec![
                "store",
                "add",
                "my_store",
                &remote_store_dir.path().to_string_lossy(),
            ],
            project_dir_a.path(),
            home_dir.path(),
        );
        run_shadowbox(
            vec!["map", "*", "my_store"],
            project_dir_a.path(),
            home_dir.path(),
        );

        let test_file = "private.txt";
        fs::write(project_dir_a.path().join(test_file), "secret content").unwrap();
        run_shadowbox(
            vec!["track", test_file],
            project_dir_a.path(),
            home_dir.path(),
        );
        run_shadowbox(vec!["push"], project_dir_a.path(), home_dir.path());

        Command::new("git")
            .args(["init"])
            .current_dir(project_dir_b.path())
            .status()
            .unwrap();
        Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                &project_remote_dir.path().to_string_lossy(),
            ])
            .current_dir(project_dir_b.path())
            .status()
            .unwrap();

        let output = run_shadowbox(vec!["pull"], project_dir_b.path(), home_dir.path());
        assert!(output.status.success());
        assert!(project_dir_b.path().join(test_file).exists());
        assert_eq!(
            fs::read_to_string(project_dir_b.path().join(test_file)).unwrap(),
            "secret content"
        );
    }
}
