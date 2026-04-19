use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_link_command() {
    let home_dir = tempdir().expect("Failed to create temp home");
    let remote_store_dir = tempdir().expect("Failed to create temp remote store");
    let project_dir = tempdir().expect("Failed to create temp project");

    let binary_path = env::current_dir().unwrap().join("target/debug/shadowbox");

    // Helper to run shadowbox
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

    // 1. Setup Remotes & Projects
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
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/user/link-test",
        ])
        .current_dir(project_dir.path())
        .status()
        .unwrap();

    // 2. Setup TWO stores (to disable auto-fallback)
    run_shadowbox(
        vec![
            "store",
            "add",
            "s1",
            &remote_store_dir.path().to_string_lossy(),
        ],
        project_dir.path(),
    );
    run_shadowbox(
        vec![
            "store",
            "add",
            "s2",
            &remote_store_dir.path().to_string_lossy(),
        ],
        project_dir.path(),
    );

    // 3. Verify track fails initially (ambiguity)
    fs::write(project_dir.path().join("file.txt"), "data").unwrap();
    let out = run_shadowbox(vec!["track", "file.txt"], project_dir.path());
    assert!(!out.status.success());

    // 4. Use LINK to map current repo to s1
    let out = run_shadowbox(vec!["link", "s1"], project_dir.path());
    assert!(
        out.status.success(),
        "Link failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // 5. Verify track now succeeds
    let out = run_shadowbox(vec!["track", "file.txt"], project_dir.path());
    assert!(
        out.status.success(),
        "Track failed after link: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // 6. Verify config.toml contains the explicit mapping
    let config_path = config_home.join("shadowbox").join("config.toml");
    let content = fs::read_to_string(config_path).unwrap();
    assert!(content.contains("github.com/user/link-test"));
    assert!(content.contains("s1"));
}
