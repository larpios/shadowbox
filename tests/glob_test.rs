use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_glob_tracking() {
    let home_dir = tempdir().expect("Failed to create temp home");
    let remote_store_dir = tempdir().expect("Failed to create temp remote store");
    let project_dir = tempdir().expect("Failed to create temp project");
    let binary_path = env::current_dir().unwrap().join("target/debug/shadowbox");

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

    // 2. Configure Shadowbox
    run_shadowbox(
        vec![
            "store",
            "add",
            "my_store",
            &remote_store_dir.path().to_string_lossy(),
        ],
        project_dir.path(),
    );
    run_shadowbox(vec!["map", "**", "my_store"], project_dir.path());

    // 3. Create multiple files
    let log1 = "logs/app.log";
    let log2 = "logs/error.log";
    let other = "config.yaml";

    fs::create_dir_all(project_dir.path().join("logs")).unwrap();
    fs::write(project_dir.path().join(log1), "log 1").unwrap();
    fs::write(project_dir.path().join(log2), "log 2").unwrap();
    fs::write(project_dir.path().join(other), "config").unwrap();

    // 4. Track with glob
    let output = run_shadowbox(vec!["track", "logs/*.log"], project_dir.path());
    assert!(
        output.status.success(),
        "track failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // 5. Verify results - .shadowbox should NOT exist locally
    assert!(!project_dir.path().join(".shadowbox").exists());

    // Files should exist in the store
    let store_path = home_dir
        .path()
        .join(".local/share/shadowbox/stores/my_store/github.com/user/project");
    assert!(store_path.join(log1).exists());
    assert!(store_path.join(log2).exists());
    assert!(!store_path.join(other).exists());

    // Verify NOT in gitignore
    let gitignore_path = project_dir.path().join(".gitignore");
    if gitignore_path.exists() {
        let gitignore_content = fs::read_to_string(gitignore_path).unwrap();
        assert!(!gitignore_content.contains(log1));
        assert!(!gitignore_content.contains(log2));
    }
}
