use std::process::Command;
use tempfile::tempdir;
use std::env;
use std::fs;

#[test]
fn test_integration_step3() {
    let dir = tempdir().expect("Failed to create temp dir");
    let repo_root = dir.path();
    
    // Create .git/hooks to simulate a git repo for hook installation
    let git_hooks_dir = repo_root.join(".git").join("hooks");
    fs::create_dir_all(&git_hooks_dir).expect("Failed to create .git/hooks");

    let binary_path = env::current_dir().unwrap().join("target/debug/shadowbox");

    // 1. Initialize shadowbox
    let status = Command::new(&binary_path)
        .arg("init")
        .current_dir(repo_root)
        .status()
        .expect("Failed to execute init");
    assert!(status.success());
    assert!(repo_root.join(".shadowbox").exists());
    assert!(repo_root.join(".gitignore").exists());

    // 2. Install hooks
    let status = Command::new(&binary_path)
        .arg("hooks")
        .arg("install")
        .current_dir(repo_root)
        .status()
        .expect("Failed to execute hooks install");
    assert!(status.success());
    assert!(git_hooks_dir.join("post-checkout").exists());
    assert!(git_hooks_dir.join("post-merge").exists());

    // 3. Track a file
    let test_file = "my_artifact.log";
    fs::write(repo_root.join(test_file), "some content").unwrap();
    
    let status = Command::new(&binary_path)
        .arg("track")
        .arg(test_file)
        .current_dir(repo_root)
        .status()
        .expect("Failed to execute track");
    assert!(status.success());

    // Verify tracked in .shadowbox and .gitignore
    let shadowbox_content = fs::read_to_string(repo_root.join(".shadowbox")).unwrap();
    assert!(shadowbox_content.contains(test_file));
    let gitignore_content = fs::read_to_string(repo_root.join(".gitignore")).unwrap();
    assert!(gitignore_content.contains(test_file));

    // 4. Test Sync (manually trigger what hooks would do)
    // First, let's remove it from .gitignore to see if sync adds it back
    let gitignore_content = fs::read_to_string(repo_root.join(".gitignore")).unwrap();
    let new_gitignore = gitignore_content.replace(test_file, "");
    fs::write(repo_root.join(".gitignore"), &new_gitignore).unwrap();
    assert!(!fs::read_to_string(repo_root.join(".gitignore")).unwrap().contains(test_file));

    let status = Command::new(&binary_path)
        .arg("sync")
        .current_dir(repo_root)
        .status()
        .expect("Failed to execute sync");
    assert!(status.success());
    assert!(fs::read_to_string(repo_root.join(".gitignore")).unwrap().contains(test_file));

    // 5. Test Uninstall
    let status = Command::new(&binary_path)
        .arg("hooks")
        .arg("uninstall")
        .current_dir(repo_root)
        .status()
        .expect("Failed to execute hooks uninstall");
    assert!(status.success());
    
    // Hooks should still exist but not contain 'shadowbox sync'
    for hook in &["post-checkout", "post-merge"] {
        let content = fs::read_to_string(git_hooks_dir.join(hook)).unwrap();
        assert!(!content.contains("shadowbox sync"));
    }
}
