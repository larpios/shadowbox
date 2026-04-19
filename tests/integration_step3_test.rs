use std::env;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_integration_step3() {
    let home_dir = tempdir().expect("Failed to create temp home");
    let remote_store_dir = tempdir().expect("Failed to create temp remote store");
    let dir = tempdir().expect("Failed to create temp project dir");
    let repo_root = dir.path();

    // Create .git/hooks to simulate a git repo for hook installation
    let git_hooks_dir = repo_root.join(".git").join("hooks");
    fs::create_dir_all(&git_hooks_dir).expect("Failed to create .git/hooks");

    let binary_path = env::current_dir().unwrap().join("target/debug/shadowbox");

    // Helper to run shadowbox
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

    // 0. Setup store and mapping
    Command::new("git")
        .args(["init", "--bare"])
        .current_dir(remote_store_dir.path())
        .status()
        .unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(repo_root)
        .status()
        .unwrap();
    Command::new("git")
        .args(["remote", "add", "origin", "https://github.com/user/project"])
        .current_dir(repo_root)
        .status()
        .unwrap();

    run_shadowbox(
        vec![
            "store",
            "add",
            "my_store",
            &remote_store_dir.path().to_string_lossy(),
        ],
        repo_root,
    );
    run_shadowbox(vec!["map", "**", "my_store"], repo_root);

    // 2. Install hooks
    let output = run_shadowbox(vec!["hooks", "install"], repo_root);
    assert!(output.status.success());
    assert!(git_hooks_dir.join("post-checkout").exists());
    assert!(git_hooks_dir.join("post-merge").exists());

    // 3. Track a file
    let test_file = "my_artifact.log";
    fs::write(repo_root.join(test_file), "some content").unwrap();

    let output = run_shadowbox(vec!["track", test_file], repo_root);
    assert!(
        output.status.success(),
        "track failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify tracked in store .shadowbox and local .gitignore
    assert!(!repo_root.join(".shadowbox").exists());

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
    let gitignore_content = fs::read_to_string(repo_root.join(".gitignore")).unwrap();
    assert!(gitignore_content.contains(test_file));

    // 4. Test Sync
    let gitignore_content = fs::read_to_string(repo_root.join(".gitignore")).unwrap();
    let new_gitignore = gitignore_content.replace(test_file, "");
    fs::write(repo_root.join(".gitignore"), &new_gitignore).unwrap();
    assert!(
        !fs::read_to_string(repo_root.join(".gitignore"))
            .unwrap()
            .contains(test_file)
    );

    let output = run_shadowbox(vec!["sync"], repo_root);
    assert!(output.status.success());
    assert!(
        fs::read_to_string(repo_root.join(".gitignore"))
            .unwrap()
            .contains(test_file)
    );

    // 5. Test Uninstall
    let output = run_shadowbox(vec!["hooks", "uninstall"], repo_root);
    assert!(output.status.success());

    for hook in &["post-checkout", "post-merge"] {
        let content = fs::read_to_string(git_hooks_dir.join(hook)).unwrap();
        assert!(!content.contains("shadowbox pull"));
    }
}
