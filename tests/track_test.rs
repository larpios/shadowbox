use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_track_subcommand() {
    let home_dir = tempdir().expect("Failed to create temp home");
    let remote_store_dir = tempdir().expect("Failed to create temp remote store");
    let dir = tempdir().expect("Failed to create temp project");
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

    // Setup remotes
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

    // Track it
    let output = run_shadowbox(vec!["track", test_file], dir.path());
    assert!(
        output.status.success(),
        "Track failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify NO .shadowbox in project
    assert!(
        !dir.path().join(".shadowbox").exists(),
        ".shadowbox file should not exist!"
    );

    // Verify file copied to store
    // Path: data_home/shadowbox/stores/my_store/github.com/user/project/test.log
    let store_file_path = data_home
        .join("shadowbox/stores/my_store/github.com/user/project")
        .join(test_file);
    assert!(
        store_file_path.exists(),
        "File not found in store at {}",
        store_file_path.display()
    );
}
