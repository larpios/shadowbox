use std::env;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_store_and_mapping() {
    let home_dir = tempdir().expect("Failed to create temp home");
    let remote_dir = tempdir().expect("Failed to create temp remote");
    let project_dir = tempdir().expect("Failed to create temp project");

    // Initialize the "remote" store repository
    Command::new("git")
        .args(["init", "--bare"])
        .current_dir(remote_dir.path())
        .status()
        .expect("Failed to init bare repo");

    let binary_path = env::current_dir().unwrap().join("target/debug/shadowbox");

    // Helper to run shadowbox with isolated home
    let run_shadowbox = |args: Vec<&str>, current_dir: &std::path::Path| {
        Command::new(&binary_path)
            .args(args)
            .current_dir(current_dir)
            .env("HOME", home_dir.path())
            .env("XDG_CONFIG_HOME", home_dir.path().join(".config"))
            .env("XDG_DATA_HOME", home_dir.path().join(".local/share"))
            .output()
            .expect("Failed to execute shadowbox")
    };

    // 1. Add store
    let remote_url = remote_dir.path().to_string_lossy();
    let output = run_shadowbox(
        vec!["store", "add", "my_store", &remote_url],
        project_dir.path(),
    );
    assert!(
        output.status.success(),
        "stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // 2. Add mapping
    let output = run_shadowbox(vec!["map", "my_project", "my_store"], project_dir.path());
    assert!(output.status.success());

    // 3. Verify config.toml
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
    assert!(found_config, "config.toml not found in temp home");
}
