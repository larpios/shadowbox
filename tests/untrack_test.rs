use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_untrack_subcommand() {
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
    run_shadowbox(vec!["map", "**", "my_store"], dir.path());

    let test_file = "test.log";
    fs::write(dir.path().join(test_file), "content").unwrap();

    // Track it
    run_shadowbox(vec!["track", test_file], dir.path());

    // Untrack it
    let output = run_shadowbox(vec!["untrack", test_file], dir.path());
    assert!(output.status.success());

    // Verify removed from store
    let store_file_path = home_dir.path()
        .join(".local/share/shadowbox/stores/my_store/github.com/user/project")
        .join(test_file);
    assert!(!store_file_path.exists());

    // Verify NOT in gitignore
    let gitignore_path = dir.path().join(".gitignore");
    if gitignore_path.exists() {
        let gitignore_content = fs::read_to_string(gitignore_path).unwrap();
        assert!(!gitignore_content.contains(test_file));
    }
}
