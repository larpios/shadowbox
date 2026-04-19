use std::process::Command;
use tempfile::tempdir;
use std::env;
use std::fs;
use std::path::Path;

#[test]
fn test_default_store_fallback() {
    let home_dir = tempdir().expect("Failed to create temp home");
    let remote_store_dir = tempdir().expect("Failed to create temp remote store");
    let project_dir = tempdir().expect("Failed to create temp project");

    let binary_path = env::current_dir().unwrap().join("target/debug/shadowbox");

    // 1. Setup Remotes
    Command::new("git").args(["init", "--bare"]).current_dir(remote_store_dir.path()).status().unwrap();
    Command::new("git").args(["init"]).current_dir(project_dir.path()).status().unwrap();
    Command::new("git").args(["remote", "add", "origin", "https://github.com/user/project"]).current_dir(project_dir.path()).status().unwrap();

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

    // 2. Add ONLY ONE store (No explicit mapping!)
    let out = run_shadowbox(vec!["store", "add", "only_one", &remote_store_dir.path().to_string_lossy()], project_dir.path());
    assert!(out.status.success());

    // 3. Try to track a file - should succeed because of the fallback
    fs::write(project_dir.path().join(".env"), "SECRET=true").unwrap();
    let out = run_shadowbox(vec!["track", ".env"], project_dir.path());
    assert!(out.status.success(), "Track should have worked via default store fallback. stderr: {}", String::from_utf8_lossy(&out.stderr));

    // 4. Verify in store
    let mut found = false;
    for entry in walkdir::WalkDir::new(home_dir.path()) {
        let entry = entry.unwrap();
        if entry.file_name() == ".shadowbox" {
            let content = fs::read_to_string(entry.path()).unwrap();
            assert!(content.contains(".env"));
            found = true;
        }
    }
    assert!(found, ".shadowbox index not found in store");
}
