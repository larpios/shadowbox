use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_distributed_push_pull() {
    let home_dir = tempdir().expect("Failed to create temp home");
    let remote_store_dir = tempdir().expect("Failed to create temp remote store");
    let project_remote_dir = tempdir().expect("Failed to create temp project remote");

    let project_dir_a = tempdir().expect("Failed to create temp project A");
    let project_dir_b = tempdir().expect("Failed to create temp project B");

    let binary_path = env::current_dir().unwrap().join("target/debug/shadowbox");

    // 1. Setup Remotes
    Command::new("git")
        .args(["init", "--bare"])
        .current_dir(remote_store_dir.path())
        .status()
        .unwrap();
    Command::new("git")
        .args(["init", "--bare"])
        .current_dir(project_remote_dir.path())
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

    // 2. Setup Project A
    Command::new("git")
        .args(["init"])
        .current_dir(project_dir_a.path())
        .status()
        .unwrap();
    Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            &project_remote_dir.path().to_string_lossy(),
        ])
        .current_dir(project_dir_a.path())
        .status()
        .unwrap();

    // 3. Configure Shadowbox
    run_shadowbox(
        vec![
            "store",
            "add",
            "my_store",
            &remote_store_dir.path().to_string_lossy(),
        ],
        project_dir_a.path(),
    );
    // Map the project remote URL (normalized) to the store
    // get_repo_id() normalizes it. Let's just use a glob that matches it.
    run_shadowbox(vec!["map", "*", "my_store"], project_dir_a.path());

    // 4. Track and Push from A
    let test_file = "private.txt";
    fs::write(project_dir_a.path().join(test_file), "secret content").unwrap();

    run_shadowbox(vec!["init"], project_dir_a.path());
    run_shadowbox(vec!["track", test_file], project_dir_a.path());

    let output = run_shadowbox(vec!["push"], project_dir_a.path());
    assert!(
        output.status.success(),
        "Push failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // 5. Setup Project B (simulating a fresh clone)
    Command::new("git")
        .args(["init"])
        .current_dir(project_dir_b.path())
        .status()
        .unwrap();
    Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            &project_remote_dir.path().to_string_lossy(),
        ])
        .current_dir(project_dir_b.path())
        .status()
        .unwrap();

    // 6. Pull in Project B
    let output = run_shadowbox(vec!["pull"], project_dir_b.path());
    assert!(
        output.status.success(),
        "Pull failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // 7. Verify file restored in B
    assert!(project_dir_b.path().join(test_file).exists());
    assert_eq!(
        fs::read_to_string(project_dir_b.path().join(test_file)).unwrap(),
        "secret content"
    );

    // Verify .gitignore updated in B
    let gitignore = fs::read_to_string(project_dir_b.path().join(".gitignore")).unwrap();
    assert!(gitignore.contains(test_file));
}
