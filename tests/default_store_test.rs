use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_default_store_fallback() {
    let home_dir = tempdir().expect("Failed to create temp home");
    let remote_store_dir = tempdir().expect("Failed to create temp remote store");
    let project_dir = tempdir().expect("Failed to create temp project");

    // Get the absolute path to the shadowbox binary
    let binary_path = env::current_dir().unwrap().join("target/debug/shadowbox");

    // Ensure binary is built once
    let _ = Command::new("cargo")
        .arg("build")
        .status()
        .expect("Failed to build shadowbox");

    // 1. Setup Remotes
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

    // 2. Add ONLY ONE store (No explicit mapping!)
    let out = run_shadowbox(
        vec![
            "store",
            "add",
            "only_one",
            &remote_store_dir.path().to_string_lossy(),
        ],
        project_dir.path(),
    );
    if !out.status.success() {
        println!("store add stdout: {}", String::from_utf8_lossy(&out.stdout));
        println!("store add stderr: {}", String::from_utf8_lossy(&out.stderr));
    }
    assert!(
        out.status.success(),
        "store add failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Verify config.toml was actually created where we expect
    let expected_config = config_home.join("shadowbox").join("config.toml");
    if !expected_config.exists() {
        println!("store add stdout: {}", String::from_utf8_lossy(&out.stdout));
        println!("store add stderr: {}", String::from_utf8_lossy(&out.stderr));
    }
    assert!(
        expected_config.exists(),
        "config.toml should exist at {}",
        expected_config.display()
    );
    let config_content = fs::read_to_string(&expected_config).unwrap();
    assert!(
        config_content.contains("only_one"),
        "config.toml should contain the store name"
    );

    // 3. Try to track a file - should succeed because of the fallback
    fs::write(project_dir.path().join(".env"), "SECRET=true").unwrap();
    let out = run_shadowbox(vec!["track", ".env"], project_dir.path());
    println!("Track stdout: {}", String::from_utf8_lossy(&out.stdout));
    println!("Track stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(
        out.status.success(),
        "Track should have worked via default store fallback. stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // 4. Verify in store
    let store_file_path = data_home.join("shadowbox/stores/only_one/github.com/user/project/.env");
    assert!(store_file_path.exists(), "Tracked file should exist in store at {}", store_file_path.display());
}
