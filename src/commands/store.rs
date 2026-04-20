use crate::config::{Config, StoreConfig, data_dir};
use std::fs;
use std::process::Command;

pub fn add(name: &str, url: &str, force: bool) -> std::io::Result<()> {
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
        return Err(std::io::Error::other(
            "Failed to clone store repository. Check your URL/permissions and try again.",
        ));
    }

    config
        .stores
        .insert(name.to_string(), StoreConfig { url: git_url });
    config.save()?;
    println!("Store '{}' added.", name);
    Ok(())
}

pub fn list() -> std::io::Result<()> {
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
    config.save()?;
    println!("Mapped '{}' to '{}'.", pattern, store);
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
    fn test_store_and_mapping() {
        let home_dir = tempdir().expect("Failed to create temp home");
        let remote_dir = tempdir().expect("Failed to create temp remote");
        let project_dir = tempdir().expect("Failed to create temp project");

        Command::new("git")
            .args(["init", "--bare"])
            .current_dir(remote_dir.path())
            .status()
            .unwrap();

        run_shadowbox(
            vec![
                "store",
                "add",
                "my_store",
                &remote_dir.path().to_string_lossy(),
            ],
            project_dir.path(),
            home_dir.path(),
        );
        run_shadowbox(
            vec!["map", "my_project", "my_store"],
            project_dir.path(),
            home_dir.path(),
        );

        let mut found_config = false;
        for entry in walkdir::WalkDir::new(home_dir.path()) {
            let entry = entry.unwrap();
            if entry.file_name() == "config.toml" {
                let content = fs::read_to_string(entry.path()).unwrap();
                assert!(content.contains("my_store"));
                assert!(content.contains("my_project"));
                found_config = true;
                break;
            }
        }
        assert!(found_config);
    }

    #[test]
    fn test_multi_store_ambiguity() {
        let home_dir = tempdir().expect("Failed to create temp home");
        let remote_1 = tempdir().expect("Failed to create remote 1");
        let remote_2 = tempdir().expect("Failed to create remote 2");
        let project_dir = tempdir().expect("Failed to create project");

        Command::new("git")
            .args(["init", "--bare"])
            .current_dir(remote_1.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["init", "--bare"])
            .current_dir(remote_2.path())
            .status()
            .unwrap();

        run_shadowbox(
            vec![
                "store",
                "add",
                "personal",
                &remote_1.path().to_string_lossy(),
            ],
            project_dir.path(),
            home_dir.path(),
        );
        run_shadowbox(
            vec!["store", "add", "work", &remote_2.path().to_string_lossy()],
            project_dir.path(),
            home_dir.path(),
        );

        Command::new("git")
            .args(["init"])
            .current_dir(project_dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                "https://github.com/user/ambiguity-test",
            ])
            .current_dir(project_dir.path())
            .status()
            .unwrap();

        fs::write(project_dir.path().join("secret.txt"), "data").unwrap();
        let output = run_shadowbox(
            vec!["track", "secret.txt"],
            project_dir.path(),
            home_dir.path(),
        );
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("Link a vault first"));

        run_shadowbox(
            vec!["map", "*", "work"],
            project_dir.path(),
            home_dir.path(),
        );
        let output = run_shadowbox(
            vec!["track", "secret.txt"],
            project_dir.path(),
            home_dir.path(),
        );
        assert!(output.status.success());
    }

    #[test]
    fn test_cross_project_isolation_in_same_store() {
        let home_dir = tempdir().expect("Failed to create temp home");
        let remote_store = tempdir().expect("Failed to create remote store");
        let project_1_dir = tempdir().expect("Failed to create project 1");
        let project_2_dir = tempdir().expect("Failed to create project 2");

        Command::new("git")
            .args(["init", "--bare"])
            .current_dir(remote_store.path())
            .status()
            .unwrap();
        run_shadowbox(
            vec![
                "store",
                "add",
                "shared_vault",
                &remote_store.path().to_string_lossy(),
            ],
            project_1_dir.path(),
            home_dir.path(),
        );

        Command::new("git")
            .args(["init"])
            .current_dir(project_1_dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                "https://github.com/user/project-1",
            ])
            .current_dir(project_1_dir.path())
            .status()
            .unwrap();
        fs::write(project_1_dir.path().join("p1.txt"), "p1-content").unwrap();
        run_shadowbox(
            vec!["track", "p1.txt"],
            project_1_dir.path(),
            home_dir.path(),
        );
        run_shadowbox(vec!["push"], project_1_dir.path(), home_dir.path());

        Command::new("git")
            .args(["init"])
            .current_dir(project_2_dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                "https://github.com/user/project-2",
            ])
            .current_dir(project_2_dir.path())
            .status()
            .unwrap();
        fs::write(project_2_dir.path().join("p2.txt"), "p2-content").unwrap();
        run_shadowbox(
            vec!["track", "p2.txt"],
            project_2_dir.path(),
            home_dir.path(),
        );
        run_shadowbox(vec!["push"], project_2_dir.path(), home_dir.path());

        let p1_restore_dir = tempdir().expect("Failed to create p1 restore");
        Command::new("git")
            .args(["init"])
            .current_dir(p1_restore_dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                "https://github.com/user/project-1",
            ])
            .current_dir(p1_restore_dir.path())
            .status()
            .unwrap();
        run_shadowbox(vec!["pull"], p1_restore_dir.path(), home_dir.path());

        assert!(p1_restore_dir.path().join("p1.txt").exists());
        assert!(!p1_restore_dir.path().join("p2.txt").exists());
    }
}
