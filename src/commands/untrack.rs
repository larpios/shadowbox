use crate::config::Config;
use crate::git::{get_repo_id, get_repo_root};
use crate::utils::get_vault_project_path;
use std::fs;
use std::path::Path;

pub fn run(path: &str) -> std::io::Result<()> {
    let repo_root = get_repo_root()?;
    let abs_path = std::path::absolute(Path::new(path))?;
    let rel_to_root = abs_path
        .strip_prefix(&repo_root)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Outside repo"))?;

    let config = Config::load()?;
    let repo_id = get_repo_id()?;
    let vault_project_path = get_vault_project_path(&config, &repo_id)?;
    let target = vault_project_path.join(rel_to_root);

    if let Ok(metadata) = fs::symlink_metadata(&target) {
        if metadata.is_dir() {
            fs::remove_dir_all(target)?;
        } else {
            fs::remove_file(target)?;
        }
    }
    println!("Untracked {}", rel_to_root.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use tempfile::tempdir;

    #[test]
    fn test_untrack_subcommand() {
        let home_dir = tempdir().expect("Failed to create temp home");
        let remote_store_dir = tempdir().expect("Failed to create temp remote store");
        let dir = tempdir().expect("Failed to create temp project");
        let binary_path = env::current_dir().unwrap().join("target/debug/shadowbox");

        let config_home = home_dir.path().join(".config");
        let data_home = home_dir.path().join(".local/share");

        let run_shadowbox = |args: Vec<&str>, current_dir: &Path| {
            Command::new(&binary_path)
                .args(args)
                .current_dir(current_dir)
                .env("HOME", home_dir.path())
                .env("XDG_CONFIG_HOME", &config_home)
                .env("XDG_DATA_HOME", &data_home)
                .output()
                .expect("Failed to execute shadowbox")
        };

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
        );

        let test_file = "test.log";
        fs::write(dir.path().join(test_file), "content").unwrap();

        run_shadowbox(vec!["track", test_file], dir.path());
        let store_file_path = data_home
            .join("shadowbox/stores/my_store/github.com/user/project")
            .join(test_file);
        assert!(store_file_path.exists());

        let output = run_shadowbox(vec!["untrack", test_file], dir.path());
        assert!(output.status.success());
        assert!(!store_file_path.exists());
    }
}
