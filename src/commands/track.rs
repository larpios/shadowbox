use crate::config::Config;
use crate::git::{get_repo_id, get_repo_root};
use crate::utils::{copy_recursive, get_store_dir, get_vault_project_path, is_lfs_available};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn run(path_patterns: &[String], follow_links: bool) -> std::io::Result<()> {
    let mut tracked = Vec::new();
    for pattern in path_patterns {
        tracked.append(&mut track_file(pattern, follow_links)?);
    }
    for p in tracked {
        println!("Tracked {}", p.display());
    }
    Ok(())
}

fn track_file(path_pattern: &str, follow_links: bool) -> std::io::Result<Vec<PathBuf>> {
    let config = Config::load()?;
    let repo_id = get_repo_id()?;
    let vault_project_path = get_vault_project_path(&config, &repo_id)?;
    let store_dir = get_store_dir(&config, &repo_id)?;
    let repo_root = get_repo_root()?;
    let canonical_repo_root = fs::canonicalize(&repo_root).unwrap_or_else(|_| repo_root.clone());

    let mut tracked = Vec::new();
    for entry in glob::glob(path_pattern)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?
    {
        let local_path = entry.map_err(|e| std::io::Error::other(e.to_string()))?;
        let _ = fs::symlink_metadata(&local_path)?;

        // For destination path in vault, we always use the path as specified (relative to repo root)
        let abs_for_rel = std::path::absolute(&local_path)?;

        // Try to strip prefix from both original repo_root and canonicalized repo_root
        // This helps when the repo path contains symlinks (like /var -> /private/var on macOS)
        let rel_to_root = abs_for_rel
            .strip_prefix(&repo_root)
            .or_else(|_| abs_for_rel.strip_prefix(&canonical_repo_root))
            .map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!(
                        "Path {} is outside repository {}",
                        abs_for_rel.display(),
                        repo_root.display()
                    ),
                )
            })?;

        // For source path to copy from, we follow links only if follow_links is true
        let src_path = if follow_links {
            fs::canonicalize(&local_path)?
        } else {
            abs_for_rel.clone()
        };

        let dest = vault_project_path.join(rel_to_root);
        println!("  [TRACK] {} -> vault", rel_to_root.display());
        copy_recursive(&src_path, &dest, follow_links)?;

        // LFS support
        if is_lfs_available() {
            // Check metadata of the source (follows links if follow_links is true due to canonicalize above)
            let metadata = fs::metadata(&src_path)?;
            if metadata.is_file() && metadata.len() > 5 * 1024 * 1024 {
                // Only track in LFS if it's not a symlink OR we followed it
                let is_symlink = fs::symlink_metadata(&src_path)?.is_symlink();
                if !is_symlink || follow_links {
                    // Track in LFS in the store
                    let rel_in_store = PathBuf::from(&repo_id).join(rel_to_root);
                    let _ = Command::new("git-lfs")
                        .current_dir(&store_dir)
                        .args(["track", &rel_in_store.to_string_lossy()])
                        .status();
                }
            }
        }

        tracked.push(rel_to_root.to_path_buf());
    }
    Ok(tracked)
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use tempfile::tempdir;

    fn run_shadowbox(
        args: Vec<&str>,
        current_dir: &Path,
        home_dir: &Path,
        bin_dir: Option<&Path>,
    ) -> std::process::Output {
        let binary_path = env::current_dir().unwrap().join("target/debug/shadowbox");
        let mut cmd = Command::new(&binary_path);
        cmd.args(args)
            .current_dir(current_dir)
            .env("HOME", home_dir)
            .env("XDG_CONFIG_HOME", home_dir.join(".config"))
            .env("XDG_DATA_HOME", home_dir.join(".local/share"));

        if let Some(bin) = bin_dir {
            let mut path_val = bin.to_string_lossy().to_string();
            if let Ok(existing_path) = env::var("PATH") {
                path_val = format!("{}:{}", path_val, existing_path);
            }
            cmd.env("PATH", path_val);
        }

        let output = cmd.output().expect("Failed to execute shadowbox");
        if !output.status.success() {
            println!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
            println!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
        } else {
            println!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
        }
        output
    }

    #[test]
    fn test_track_subcommand() {
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
            None,
        );

        let test_file = "test.log";
        fs::write(dir.path().join(test_file), "content").unwrap();

        let output = run_shadowbox(vec!["track", test_file], dir.path(), home_dir.path(), None);
        assert!(output.status.success());
        assert!(!dir.path().join(".shadowbox").exists());
        assert!(
            home_dir
                .path()
                .join(".local/share/shadowbox/stores/my_store/github.com/user/project/test.log")
                .exists()
        );
    }

    #[test]
    fn test_directory_tracking_recursive() {
        let home_dir = tempdir().expect("Failed to create temp home");
        let remote_store_dir = tempdir().expect("Failed to create temp remote store");
        let project_a_dir = tempdir().expect("Failed to create project A");
        let project_b_dir = tempdir().expect("Failed to create project B");

        Command::new("git")
            .args(["init", "--bare"])
            .current_dir(remote_store_dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(project_a_dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                "https://github.com/user/dir-test",
            ])
            .current_dir(project_a_dir.path())
            .status()
            .unwrap();

        run_shadowbox(
            vec![
                "store",
                "add",
                "vault",
                &remote_store_dir.path().to_string_lossy(),
            ],
            project_a_dir.path(),
            home_dir.path(),
            None,
        );

        let secret_dir = "secrets/configs";
        fs::create_dir_all(project_a_dir.path().join(secret_dir)).unwrap();
        fs::write(
            project_a_dir.path().join(secret_dir).join("key.txt"),
            "private-key",
        )
        .unwrap();
        fs::write(
            project_a_dir.path().join(secret_dir).join("cert.pem"),
            "cert-data",
        )
        .unwrap();

        run_shadowbox(
            vec!["track", "secrets"],
            project_a_dir.path(),
            home_dir.path(),
            None,
        );
        run_shadowbox(vec!["push"], project_a_dir.path(), home_dir.path(), None);

        Command::new("git")
            .args(["init"])
            .current_dir(project_b_dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                "https://github.com/user/dir-test",
            ])
            .current_dir(project_b_dir.path())
            .status()
            .unwrap();

        let output = run_shadowbox(vec!["pull"], project_b_dir.path(), home_dir.path(), None);
        assert!(output.status.success());
        assert!(
            project_b_dir
                .path()
                .join("secrets/configs/key.txt")
                .exists()
        );
        assert_eq!(
            fs::read_to_string(project_b_dir.path().join("secrets/configs/key.txt")).unwrap(),
            "private-key"
        );
    }

    #[test]
    fn test_glob_tracking() {
        let home_dir = tempdir().expect("Failed to create temp home");
        let remote_store_dir = tempdir().expect("Failed to create temp remote store");
        let project_dir = tempdir().expect("Failed to create temp project");

        Command::new("git")
            .args(["init", "--bare"])
            .current_dir(remote_store_dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(project_dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["remote", "add", "origin", "https://github.com/user/project"])
            .current_dir(project_dir.path())
            .status()
            .unwrap();

        run_shadowbox(
            vec![
                "store",
                "add",
                "my_store",
                &remote_store_dir.path().to_string_lossy(),
            ],
            project_dir.path(),
            home_dir.path(),
            None,
        );
        run_shadowbox(
            vec!["map", "**", "my_store"],
            project_dir.path(),
            home_dir.path(),
            None,
        );

        fs::create_dir_all(project_dir.path().join("logs")).unwrap();
        fs::write(project_dir.path().join("logs/app.log"), "log 1").unwrap();
        fs::write(project_dir.path().join("logs/error.log"), "log 2").unwrap();

        let output = run_shadowbox(
            vec!["track", "logs/*.log"],
            project_dir.path(),
            home_dir.path(),
            None,
        );
        assert!(output.status.success());
        let store_path = home_dir
            .path()
            .join(".local/share/shadowbox/stores/my_store/github.com/user/project");
        assert!(store_path.join("logs/app.log").exists());
        assert!(store_path.join("logs/error.log").exists());
    }

    #[test]
    fn test_lfs_detection_and_tracking() {
        let home_dir = tempdir().expect("Failed to create temp home");
        let remote_store_dir = tempdir().expect("Failed to create temp remote store");
        let project_dir = tempdir().expect("Failed to create temp project");
        let bin_dir = tempdir().expect("Failed to create temp bin dir");

        let lfs_shim = bin_dir.path().join("git-lfs");
        #[cfg(unix)]
        {
            fs::write(
                &lfs_shim,
                r#"#!/bin/sh
case "$1" in
    track) echo "$2 filter=lfs" > LFS_TRACKED_THIS ;;
    version) echo "git-lfs/3.0.0" ;;
esac
exit 0
"#,
            )
            .unwrap();
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&lfs_shim).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&lfs_shim, perms).unwrap();
        }

        Command::new("git")
            .args(["init", "--bare"])
            .current_dir(remote_store_dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(project_dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["remote", "add", "origin", "https://github.com/user/project"])
            .current_dir(project_dir.path())
            .status()
            .unwrap();

        let out = run_shadowbox(
            vec![
                "store",
                "add",
                "my_store",
                &remote_store_dir.path().to_string_lossy(),
            ],
            project_dir.path(),
            home_dir.path(),
            Some(bin_dir.path()),
        );
        assert!(out.status.success());

        let out = run_shadowbox(
            vec!["map", "github.com/user/project", "my_store"],
            project_dir.path(),
            home_dir.path(),
            Some(bin_dir.path()),
        );
        assert!(out.status.success());

        let large_file = "model.bin";
        let large_data = vec![0u8; 6 * 1024 * 1024];
        fs::write(project_dir.path().join(large_file), &large_data).unwrap();

        let out = run_shadowbox(
            vec!["track", large_file],
            project_dir.path(),
            home_dir.path(),
            Some(bin_dir.path()),
        );
        assert!(out.status.success());

        let out = run_shadowbox(
            vec!["push"],
            project_dir.path(),
            home_dir.path(),
            Some(bin_dir.path()),
        );
        assert!(out.status.success());

        let mut tracked_file_path = None;
        for entry in walkdir::WalkDir::new(home_dir.path()) {
            let entry = entry.unwrap();
            if entry.file_name() == "LFS_TRACKED_THIS" {
                tracked_file_path = Some(entry.path().to_path_buf());
                break;
            }
        }
        let attributes =
            fs::read_to_string(tracked_file_path.expect("LFS_TRACKED_THIS should exist")).unwrap();
        assert!(attributes.contains(large_file));
    }

    #[test]
    fn test_default_store_fallback() {
        let home_dir = tempdir().expect("Failed to create temp home");
        let remote_store_dir = tempdir().expect("Failed to create temp remote store");
        let project_dir = tempdir().expect("Failed to create temp project");

        Command::new("git")
            .args(["init", "--bare"])
            .current_dir(remote_store_dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(project_dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["remote", "add", "origin", "https://github.com/user/project"])
            .current_dir(project_dir.path())
            .status()
            .unwrap();

        run_shadowbox(
            vec![
                "store",
                "add",
                "only_one",
                &remote_store_dir.path().to_string_lossy(),
            ],
            project_dir.path(),
            home_dir.path(),
            None,
        );

        fs::write(project_dir.path().join(".env"), "SECRET=true").unwrap();
        let out = run_shadowbox(
            vec!["track", ".env"],
            project_dir.path(),
            home_dir.path(),
            None,
        );
        assert!(out.status.success());
        assert!(
            home_dir
                .path()
                .join(".local/share/shadowbox/stores/only_one/github.com/user/project/.env")
                .exists()
        );
    }

    #[test]
    fn test_symlink_tracking() {
        let home_dir = tempdir().expect("Failed to create temp home");
        let remote_store_dir = tempdir().expect("Failed to create temp remote store");
        let project_dir = tempdir().expect("Failed to create temp project");

        Command::new("git")
            .args(["init", "--bare"])
            .current_dir(remote_store_dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(project_dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["remote", "add", "origin", "https://github.com/user/project"])
            .current_dir(project_dir.path())
            .status()
            .unwrap();

        run_shadowbox(
            vec![
                "store",
                "add",
                "my_store",
                &remote_store_dir.path().to_string_lossy(),
            ],
            project_dir.path(),
            home_dir.path(),
            None,
        );

        // Create a target file
        let target_file = "target.txt";
        fs::write(project_dir.path().join(target_file), "original content").unwrap();

        // Create a symlink
        let link_file = "link.txt";
        #[cfg(unix)]
        std::os::unix::fs::symlink(target_file, project_dir.path().join(link_file)).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(target_file, project_dir.path().join(link_file))
            .unwrap();

        let output = run_shadowbox(
            vec!["track", link_file],
            project_dir.path(),
            home_dir.path(),
            None,
        );
        assert!(output.status.success());

        let store_path = home_dir
            .path()
            .join(".local/share/shadowbox/stores/my_store/github.com/user/project/link.txt");

        assert!(
            fs::symlink_metadata(&store_path).is_ok(),
            "Link should exist in store"
        );
        let meta = fs::symlink_metadata(&store_path).unwrap();
        assert!(meta.is_symlink(), "Tracked file should be a symlink");

        let target = fs::read_link(&store_path).unwrap();
        assert_eq!(target.to_str().unwrap(), target_file);
    }

    #[test]
    fn test_track_symlink_to_dir_follow() {
        let home_dir = tempdir().expect("Failed to create temp home");
        let remote_store_dir = tempdir().expect("Failed to create temp remote store");
        let project_dir = tempdir().expect("Failed to create temp project");

        Command::new("git")
            .args(["init", "--bare"])
            .current_dir(remote_store_dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(project_dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["remote", "add", "origin", "https://github.com/user/project"])
            .current_dir(project_dir.path())
            .status()
            .unwrap();

        run_shadowbox(
            vec![
                "store",
                "add",
                "my_store",
                &remote_store_dir.path().to_string_lossy(),
            ],
            project_dir.path(),
            home_dir.path(),
            None,
        );

        // Create a directory with a file
        let real_dir = "real_dir";
        fs::create_dir_all(project_dir.path().join(real_dir)).unwrap();
        fs::write(
            project_dir.path().join(real_dir).join("file.txt"),
            "content",
        )
        .unwrap();

        // Create a symlink to that directory
        let link_dir = "link_dir";
        #[cfg(unix)]
        std::os::unix::fs::symlink(real_dir, project_dir.path().join(link_dir)).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(real_dir, project_dir.path().join(link_dir)).unwrap();

        let output = run_shadowbox(
            vec!["track", "--follow-links", link_dir],
            project_dir.path(),
            home_dir.path(),
            None,
        );
        assert!(output.status.success());

        let store_path = home_dir
            .path()
            .join(".local/share/shadowbox/stores/my_store/github.com/user/project/link_dir");

        assert!(
            store_path.is_dir(),
            "link_dir should be a directory in store (followed)"
        );
        assert!(
            store_path.join("file.txt").exists(),
            "file.txt should exist in store"
        );
    }

    #[test]
    fn test_glob_symlink_tracking() {
        let home_dir = tempdir().expect("Failed to create temp home");
        let remote_store_dir = tempdir().expect("Failed to create temp remote store");
        let project_dir = tempdir().expect("Failed to create temp project");

        Command::new("git")
            .args(["init", "--bare"])
            .current_dir(remote_store_dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(project_dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["remote", "add", "origin", "https://github.com/user/project"])
            .current_dir(project_dir.path())
            .status()
            .unwrap();

        run_shadowbox(
            vec![
                "store",
                "add",
                "my_store",
                &remote_store_dir.path().to_string_lossy(),
            ],
            project_dir.path(),
            home_dir.path(),
            None,
        );

        // Create a target file
        let target_file = "target.txt";
        fs::write(project_dir.path().join(target_file), "original content").unwrap();

        // Create a symlink
        let link_file = "link.txt";
        #[cfg(unix)]
        std::os::unix::fs::symlink(target_file, project_dir.path().join(link_file)).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(target_file, project_dir.path().join(link_file))
            .unwrap();

        let output = run_shadowbox(
            vec!["track", "*.txt"],
            project_dir.path(),
            home_dir.path(),
            None,
        );
        assert!(output.status.success());

        let store_path = home_dir
            .path()
            .join(".local/share/shadowbox/stores/my_store/github.com/user/project/link.txt");

        assert!(
            fs::symlink_metadata(&store_path).is_ok(),
            "Link should exist in store"
        );
        assert!(
            fs::symlink_metadata(&store_path).unwrap().is_symlink(),
            "Tracked file should be a symlink"
        );
    }

    #[test]
    fn test_path_mismatch_repro() {
        let temp = tempdir().expect("Failed to create temp home");
        let base_dir = temp.path().join("base");
        fs::create_dir_all(&base_dir).unwrap();

        let real_repo = base_dir.join("real_repo");
        fs::create_dir_all(&real_repo).unwrap();

        let link_repo = base_dir.join("link_repo");
        #[cfg(unix)]
        std::os::unix::fs::symlink("real_repo", &link_repo).unwrap();

        #[cfg(unix)]
        {
            Command::new("git")
                .args(["init"])
                .current_dir(&real_repo)
                .status()
                .unwrap();
            Command::new("git")
                .args(["remote", "add", "origin", "https://github.com/user/project"])
                .current_dir(&real_repo)
                .status()
                .unwrap();

            let home_dir = temp.path().join("home");
            let remote_store_dir = temp.path().join("remote_store");
            fs::create_dir_all(&remote_store_dir).unwrap();
            Command::new("git")
                .args(["init", "--bare"])
                .current_dir(&remote_store_dir)
                .status()
                .unwrap();

            run_shadowbox(
                vec![
                    "store",
                    "add",
                    "my_store",
                    &remote_store_dir.to_string_lossy(),
                ],
                &link_repo,
                &home_dir,
                None,
            );

            let test_file = "test.txt";
            fs::write(real_repo.join(test_file), "content").unwrap();

            let output = run_shadowbox(vec!["track", test_file], &link_repo, &home_dir, None);
            assert!(
                output.status.success(),
                "Track should succeed even through a symlinked repo path"
            );
        }
    }

    #[test]
    fn test_glob_through_symlink_dir() {
        let home_dir = tempdir().expect("Failed to create temp home");
        let remote_store_dir = tempdir().expect("Failed to create temp remote store");
        let project_dir = tempdir().expect("Failed to create temp project");

        Command::new("git")
            .args(["init", "--bare"])
            .current_dir(remote_store_dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(project_dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["remote", "add", "origin", "https://github.com/user/project"])
            .current_dir(project_dir.path())
            .status()
            .unwrap();

        run_shadowbox(
            vec![
                "store",
                "add",
                "my_store",
                &remote_store_dir.path().to_string_lossy(),
            ],
            project_dir.path(),
            home_dir.path(),
            None,
        );

        let real_dir = "real_dir";
        fs::create_dir_all(project_dir.path().join(real_dir)).unwrap();
        fs::write(
            project_dir.path().join(real_dir).join("file.txt"),
            "content",
        )
        .unwrap();

        let link_dir = "link_dir";
        #[cfg(unix)]
        std::os::unix::fs::symlink(real_dir, project_dir.path().join(link_dir)).unwrap();

        let output = run_shadowbox(
            vec!["track", "link_dir/*.txt"],
            project_dir.path(),
            home_dir.path(),
            None,
        );
        assert!(output.status.success());

        let store_path = home_dir.path().join(
            ".local/share/shadowbox/stores/my_store/github.com/user/project/link_dir/file.txt",
        );

        assert!(
            store_path.exists(),
            "file.txt should exist in store even if tracked through a symlinked directory"
        );
    }
}
