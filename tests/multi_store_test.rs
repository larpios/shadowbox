use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_multi_store_ambiguity() {
    let home_dir = tempdir().expect("Failed to create temp home");
    let remote_1 = tempdir().expect("Failed to create remote 1");
    let remote_2 = tempdir().expect("Failed to create remote 2");
    let project_dir = tempdir().expect("Failed to create project");

    let binary_path = env::current_dir().unwrap().join("target/debug/shadowbox");

    // Helper to run shadowbox
    let config_home = home_dir.path().join(".config");
    let data_home = home_dir.path().join(".local/share");

    let run_shadowbox = |args: Vec<&str>, current_dir: &Path| {
        // Ensure binary is up to date
        Command::new("cargo").args(["build"]).status().unwrap();

        Command::new(&binary_path)
            .args(args)
            .current_dir(current_dir)
            .env("HOME", home_dir.path())
            .env("XDG_CONFIG_HOME", &config_home)
            .env("XDG_DATA_HOME", &data_home)
            .output()
            .expect("Failed to execute shadowbox")
    };

    // Setup 2 stores
    Command::new("git")
        .args(["init", "--bare"])
        .current_dir(remote_1.path())
        .status()
        .unwrap();
    Command::new("git")
        .args(["init", "--bare"])
        .current_dir(remote_2.path())
        .status()
        .unwrap();

    run_shadowbox(
        vec![
            "store",
            "add",
            "personal",
            &remote_1.path().to_string_lossy(),
        ],
        project_dir.path(),
    );
    run_shadowbox(
        vec!["store", "add", "work", &remote_2.path().to_string_lossy()],
        project_dir.path(),
    );

    // Project setup
    Command::new("git")
        .args(["init"])
        .current_dir(project_dir.path())
        .status()
        .unwrap();
    Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/user/ambiguity-test",
        ])
        .current_dir(project_dir.path())
        .status()
        .unwrap();

    // Try to track without mapping - SHOULD FAIL
    fs::write(project_dir.path().join("secret.txt"), "data").unwrap();
    let output = run_shadowbox(vec!["track", "secret.txt"], project_dir.path());
    assert!(
        !output.status.success(),
        "Track should have failed because there are 2 stores and no mapping"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("Link a vault first"));

    // Map it to 'work'
    run_shadowbox(vec!["map", "*", "work"], project_dir.path());

    // Now it should succeed
    let output = run_shadowbox(vec!["track", "secret.txt"], project_dir.path());
    assert!(
        output.status.success(),
        "Track should have succeeded after mapping. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_cross_project_isolation_in_same_store() {
    let home_dir = tempdir().expect("Failed to create temp home");
    let remote_store = tempdir().expect("Failed to create remote store");
    let project_1_dir = tempdir().expect("Failed to create project 1");
    let project_2_dir = tempdir().expect("Failed to create project 2");

    let binary_path = env::current_dir().unwrap().join("target/debug/shadowbox");

    // Helper to run shadowbox
    let config_home = home_dir.path().join(".config");
    let data_home = home_dir.path().join(".local/share");

    let run_shadowbox = |args: Vec<&str>, current_dir: &Path| {
        // Ensure binary is up to date
        Command::new("cargo").args(["build"]).status().unwrap();

        Command::new(&binary_path)
            .args(args)
            .current_dir(current_dir)
            .env("HOME", home_dir.path())
            .env("XDG_CONFIG_HOME", &config_home)
            .env("XDG_DATA_HOME", &data_home)
            .output()
            .expect("Failed to execute shadowbox")
    };

    // Setup 1 store for 2 projects
    Command::new("git")
        .args(["init", "--bare"])
        .current_dir(remote_store.path())
        .status()
        .unwrap();
    run_shadowbox(
        vec![
            "store",
            "add",
            "shared_vault",
            &remote_store.path().to_string_lossy(),
        ],
        project_1_dir.path(),
    );

    // Project 1
    Command::new("git")
        .args(["init"])
        .current_dir(project_1_dir.path())
        .status()
        .unwrap();
    Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/user/project-1",
        ])
        .current_dir(project_1_dir.path())
        .status()
        .unwrap();
    fs::write(project_1_dir.path().join("p1.txt"), "p1-content").unwrap();
    run_shadowbox(vec!["track", "p1.txt"], project_1_dir.path());
    run_shadowbox(vec!["push"], project_1_dir.path());

    // Project 2
    Command::new("git")
        .args(["init"])
        .current_dir(project_2_dir.path())
        .status()
        .unwrap();
    Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/user/project-2",
        ])
        .current_dir(project_2_dir.path())
        .status()
        .unwrap();
    fs::write(project_2_dir.path().join("p2.txt"), "p2-content").unwrap();
    run_shadowbox(vec!["track", "p2.txt"], project_2_dir.path());
    run_shadowbox(vec!["push"], project_2_dir.path());

    // Restore P1 in a fresh dir
    let p1_restore_dir = tempdir().expect("Failed to create p1 restore");
    Command::new("git")
        .args(["init"])
        .current_dir(p1_restore_dir.path())
        .status()
        .unwrap();
    Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/user/project-1",
        ])
        .current_dir(p1_restore_dir.path())
        .status()
        .unwrap();
    run_shadowbox(vec!["pull"], p1_restore_dir.path());

    assert!(p1_restore_dir.path().join("p1.txt").exists());
    assert!(
        !p1_restore_dir.path().join("p2.txt").exists(),
        "P1 should NOT have pulled P2's files!"
    );
}
