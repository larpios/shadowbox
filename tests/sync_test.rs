use std::fs;
use std::process::Command;
use tempfile::tempdir;
use std::env;
use std::path::Path;

#[test]
fn test_sync_adds_missing_to_gitignore() {
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

    // Track a file (this creates the .shadowbox in the store)
    let file1 = "file1.txt";
    fs::write(dir.path().join(file1), "content").unwrap();
    run_shadowbox(vec!["track", file1], dir.path());

    // Manually remove it from .gitignore
    let gitignore_path = dir.path().join(".gitignore");
    fs::write(&gitignore_path, "").unwrap();
    assert!(!fs::read_to_string(&gitignore_path).unwrap().contains(file1));

    // Run sync
    let output = run_shadowbox(vec!["sync"], dir.path());
    assert!(output.status.success());

    // Verify .gitignore updated
    let content = fs::read_to_string(&gitignore_path).unwrap();
    assert!(content.contains(file1));
}
