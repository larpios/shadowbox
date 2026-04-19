use std::env;
use std::fs;
use std::io::Read;
use std::process::Command;
use tempfile::tempdir;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[test]
fn test_hooks_install_and_uninstall() {
    let dir = tempdir().expect("Failed to create temp dir");
    let repo_root = dir.path();
    let git_hooks_dir = repo_root.join(".git").join("hooks");
    fs::create_dir_all(&git_hooks_dir).expect("Failed to create .git/hooks");

    let binary_path = env::current_dir().unwrap().join("target/debug/shadowbox");

    // Test install
    let output = Command::new(&binary_path)
        .arg("hooks")
        .arg("install")
        .current_dir(repo_root)
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "hooks install should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    for hook_name in &["post-checkout", "post-merge"] {
        let hook_path = git_hooks_dir.join(hook_name);
        assert!(hook_path.exists(), "{} hook should exist", hook_name);

        let mut content = String::new();
        fs::File::open(&hook_path)
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        assert!(
            content.contains("shadowbox pull"),
            "{} should contain 'shadowbox pull'",
            hook_name
        );

        #[cfg(unix)]
        {
            let metadata = fs::metadata(&hook_path).unwrap();
            let permissions = metadata.permissions();
            assert!(
                permissions.mode() & 0o111 != 0,
                "{} should be executable",
                hook_name
            );
        }
    }

    // Test uninstall
    let output = Command::new(&binary_path)
        .arg("hooks")
        .arg("uninstall")
        .current_dir(repo_root)
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "hooks uninstall should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    for hook_name in &["post-checkout", "post-merge"] {
        let hook_path = git_hooks_dir.join(hook_name);
        if hook_path.exists() {
            let mut content = String::new();
            fs::File::open(&hook_path)
                .unwrap()
                .read_to_string(&mut content)
                .unwrap();
            assert!(
                !content.contains("shadowbox pull"),
                "{} should NOT contain 'shadowbox pull' after uninstall",
                hook_name
            );
        }
    }
}
