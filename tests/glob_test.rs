use std::process::Command;
use tempfile::tempdir;
use std::env;
use std::fs;
use std::path::Path;

#[test]
fn test_glob_tracking() {
    let project_dir = tempdir().expect("Failed to create temp project");
    let binary_path = env::current_dir().unwrap().join("target/debug/shadowbox");

    // 1. Setup Project
    Command::new("git").args(["init"]).current_dir(project_dir.path()).status().unwrap();
    
    // 2. Create multiple files
    let log1 = "logs/app.log";
    let log2 = "logs/error.log";
    let other = "config.yaml";
    
    fs::create_dir_all(project_dir.path().join("logs")).unwrap();
    fs::write(project_dir.path().join(log1), "log 1").unwrap();
    fs::write(project_dir.path().join(log2), "log 2").unwrap();
    fs::write(project_dir.path().join(other), "config").unwrap();

    // 3. Track with glob
    let status = Command::new(&binary_path)
        .arg("init")
        .current_dir(project_dir.path())
        .status()
        .expect("Failed to execute init");
    assert!(status.success());

    let output = Command::new(&binary_path)
        .arg("track")
        .arg("logs/*.log")
        .current_dir(project_dir.path())
        .output()
        .expect("Failed to execute track");
    assert!(output.status.success());

    // 4. Verify results
    let shadowbox_content = fs::read_to_string(project_dir.path().join(".shadowbox")).unwrap();
    assert!(shadowbox_content.contains(log1));
    assert!(shadowbox_content.contains(log2));
    assert!(!shadowbox_content.contains(other));

    let gitignore_content = fs::read_to_string(project_dir.path().join(".gitignore")).unwrap();
    assert!(gitignore_content.contains(log1));
    assert!(gitignore_content.contains(log2));
}
