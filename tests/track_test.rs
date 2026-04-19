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

    let output = run_shadowbox(vec!["track", test_file], dir.path());
    assert!(output.status.success());

    // Verify .shadowbox NOT in project
    assert!(!dir.path().join(".shadowbox").exists());

    // Verify tracked in store
    let mut shadowbox_path = None;
    for entry in walkdir::WalkDir::new(home_dir.path()) {
        let entry = entry.unwrap();
        if entry.file_name() == ".shadowbox" {
            shadowbox_path = Some(entry.path().to_path_buf());
            break;
        }
    }
    let shadowbox_path = shadowbox_path.expect(".shadowbox should exist in store");
    let shadowbox_content = fs::read_to_string(shadowbox_path).unwrap();
    assert!(shadowbox_content.contains(test_file));

    // Verify in gitignore
    let gitignore_content = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
    assert!(gitignore_content.contains(test_file));
}
