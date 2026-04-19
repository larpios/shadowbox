use std::fs;
use std::process::Command;
use tempfile::tempdir;
use std::env;
use std::path::Path;

#[test]
fn test_status_command() {
    let home_dir = tempdir().expect("Failed to create temp home");
    let remote_store_dir = tempdir().expect("Failed to create temp remote store");
    let dir = tempdir().expect("Failed to create temp project");
    let binary_path = env::current_dir().unwrap().join("target/debug/shadowbox");

    // Helper to run shadowbox
    let run_shadowbox = |args: Vec<&str>, current_dir: &Path| {
        Command::new(&binary_path)
            .args(args)
            .current_dir(current_dir)
            .env("HOME", home_dir.path())
            .env("XDG_CONFIG_HOME", home_dir.path().join(".config"))
            .env("XDG_DATA_HOME", home_dir.path().join(".local/share"))
            .output()
            .expect("Failed to execute shadowbox")
    };

    // Setup store and mapping
    Command::new("git").args(["init", "--bare"]).current_dir(remote_store_dir.path()).status().unwrap();
    Command::new("git").args(["init"]).current_dir(dir.path()).status().unwrap();
    Command::new("git").args(["remote", "add", "origin", "https://github.com/user/project"]).current_dir(dir.path()).status().unwrap();

    run_shadowbox(vec!["store", "add", "my_store", &remote_store_dir.path().to_string_lossy()], dir.path());
    run_shadowbox(vec!["map", "**", "my_store"], dir.path());

    // Create and track files
    let file1 = "file1.txt";
    fs::write(dir.path().join(file1), "content").unwrap();
    run_shadowbox(vec!["track", file1], dir.path());

    let output = run_shadowbox(vec!["status"], dir.path());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(file1));
}

#[test]
fn test_status_missing_file() {
    let home_dir = tempdir().expect("Failed to create temp home");
    let remote_store_dir = tempdir().expect("Failed to create temp remote store");
    let dir = tempdir().expect("Failed to create temp project");
    let binary_path = env::current_dir().unwrap().join("target/debug/shadowbox");

    // Helper to run shadowbox
    let run_shadowbox = |args: Vec<&str>, current_dir: &Path| {
        Command::new(&binary_path)
            .args(args)
            .current_dir(current_dir)
            .env("HOME", home_dir.path())
            .env("XDG_CONFIG_HOME", home_dir.path().join(".config"))
            .env("XDG_DATA_HOME", home_dir.path().join(".local/share"))
            .output()
            .expect("Failed to execute shadowbox")
    };

    // Setup store and mapping
    Command::new("git").args(["init", "--bare"]).current_dir(remote_store_dir.path()).status().unwrap();
    Command::new("git").args(["init"]).current_dir(dir.path()).status().unwrap();
    Command::new("git").args(["remote", "add", "origin", "https://github.com/user/project"]).current_dir(dir.path()).status().unwrap();

    run_shadowbox(vec!["store", "add", "my_store", &remote_store_dir.path().to_string_lossy()], dir.path());
    run_shadowbox(vec!["map", "**", "my_store"], dir.path());

    // Track then delete local file
    let file1 = "missing.txt";
    fs::write(dir.path().join(file1), "content").unwrap();
    run_shadowbox(vec!["track", file1], dir.path());
    fs::remove_file(dir.path().join(file1)).unwrap();

    let output = run_shadowbox(vec!["status"], dir.path());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[MISSING] missing.txt"));
}
