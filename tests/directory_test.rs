use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_directory_tracking_recursive() {
    let home_dir = tempdir().expect("Failed to create temp home");
    let remote_store_dir = tempdir().expect("Failed to create temp remote store");
    let project_a_dir = tempdir().expect("Failed to create project A");
    let project_b_dir = tempdir().expect("Failed to create project B");

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

    // Setup
    Command::new("git")
        .args(["init", "--bare"])
        .current_dir(remote_store_dir.path())
        .status()
        .unwrap();

    // Project A
    Command::new("git")
        .args(["init"])
        .current_dir(project_a_dir.path())
        .status()
        .unwrap();
    Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/user/dir-test",
        ])
        .current_dir(project_a_dir.path())
        .status()
        .unwrap();

    run_shadowbox(
        vec![
            "store",
            "add",
            "vault",
            &remote_store_dir.path().to_string_lossy(),
        ],
        project_a_dir.path(),
    );

    // Create a directory with multiple files
    let secret_dir = "secrets/configs";
    fs::create_dir_all(project_a_dir.path().join(secret_dir)).unwrap();
    fs::write(
        project_a_dir.path().join(secret_dir).join("key.txt"),
        "private-key",
    )
    .unwrap();
    fs::write(
        project_a_dir.path().join(secret_dir).join("cert.pem"),
        "cert-data",
    )
    .unwrap();

    // Track the top-level directory
    run_shadowbox(vec!["track", "secrets"], project_a_dir.path());
    run_shadowbox(vec!["push"], project_a_dir.path());

    // Project B (Restore)
    Command::new("git")
        .args(["init"])
        .current_dir(project_b_dir.path())
        .status()
        .unwrap();
    Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/user/dir-test",
        ])
        .current_dir(project_b_dir.path())
        .status()
        .unwrap();

    let output = run_shadowbox(vec!["pull"], project_b_dir.path());
    assert!(
        output.status.success(),
        "Pull failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify recursion
    assert!(
        project_b_dir
            .path()
            .join("secrets/configs/key.txt")
            .exists()
    );
    assert_eq!(
        fs::read_to_string(project_b_dir.path().join("secrets/configs/key.txt")).unwrap(),
        "private-key"
    );
    assert!(
        project_b_dir
            .path()
            .join("secrets/configs/cert.pem")
            .exists()
    );

    // Verify .gitignore contains the root of the secret dir
    let gitignore = fs::read_to_string(project_b_dir.path().join(".gitignore")).unwrap();
    assert!(gitignore.contains("secrets"));
}
