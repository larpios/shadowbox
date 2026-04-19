use std::process::Command;
use tempfile::tempdir;
use std::env;
use std::fs;
use std::path::Path;

#[test]
fn test_lfs_detection_and_tracking() {
    let home_dir = tempdir().expect("Failed to create temp home");
    let remote_store_dir = tempdir().expect("Failed to create temp remote store");
    let project_dir = tempdir().expect("Failed to create temp project");
    let bin_dir = tempdir().expect("Failed to create temp bin dir");

    let binary_path = env::current_dir().unwrap().join("target/debug/shadowbox");

    // 1. Create a fake 'git-lfs' shim to simulate LFS being installed
    let lfs_shim = bin_dir.path().join("git-lfs");
    #[cfg(unix)]
    {
        fs::write(&lfs_shim, r#"#!/bin/sh
case "$1" in
    track)
        echo "$2 filter=lfs diff=lfs merge=lfs -text" >> .gitattributes
        ;;
    version)
        echo "git-lfs/3.0.0"
        ;;
esac
exit 0
"#).unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&lfs_shim).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&lfs_shim, perms).unwrap();
    }
    // Note: For Windows, we'd need a .bat file, but this environment is unix-like.

    // 2. Setup Remotes & Project
    Command::new("git").args(["init", "--bare"]).current_dir(remote_store_dir.path()).status().unwrap();
    Command::new("git").args(["init"]).current_dir(project_dir.path()).status().unwrap();
    Command::new("git").args(["remote", "add", "origin", "https://github.com/user/project"]).current_dir(project_dir.path()).status().unwrap();

    // Helper to run shadowbox with fake PATH
    let run_shadowbox = |args: Vec<&str>, current_dir: &Path| {
        let mut path_val = bin_dir.path().to_string_lossy().to_string();
        if let Ok(existing_path) = env::var("PATH") {
            path_val = format!("{}:{}", path_val, existing_path);
        }

        Command::new(&binary_path)
            .args(args)
            .current_dir(current_dir)
            .env("HOME", home_dir.path())
            .env("XDG_CONFIG_HOME", home_dir.path().join(".config"))
            .env("XDG_DATA_HOME", home_dir.path().join(".local/share"))
            .env("PATH", path_val)
            .output()
            .expect("Failed to execute shadowbox")
    };

    // 3. Configure Shadowbox
    let out = run_shadowbox(vec!["store", "add", "my_store", &remote_store_dir.path().to_string_lossy()], project_dir.path());
    assert!(out.status.success(), "store add failed: {}", String::from_utf8_lossy(&out.stderr));

    let out = run_shadowbox(vec!["map", "github.com/user/project", "my_store"], project_dir.path());
    assert!(out.status.success(), "map failed: {}", String::from_utf8_lossy(&out.stderr));

    let out = run_shadowbox(vec!["init"], project_dir.path());
    assert!(out.status.success(), "init failed: {}", String::from_utf8_lossy(&out.stderr));

    // 4. Create a large file (6MB) and a small file (100KB)
    let large_file = "model.bin";
    let small_file = "config.json";
    
    // Efficiently create a 6MB file
    let large_data = vec![0u8; 6 * 1024 * 1024];
    fs::write(project_dir.path().join(large_file), &large_data).unwrap();
    fs::write(project_dir.path().join(small_file), "{}").unwrap();

    let out = run_shadowbox(vec!["track", large_file], project_dir.path());
    assert!(out.status.success(), "track large failed: {}", String::from_utf8_lossy(&out.stderr));

    let out = run_shadowbox(vec!["track", small_file], project_dir.path());
    assert!(out.status.success(), "track small failed: {}", String::from_utf8_lossy(&out.stderr));

    // 5. Push to store
    let output = run_shadowbox(vec!["push"], project_dir.path());
    println!("Push stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("Push stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "Push failed: {}", String::from_utf8_lossy(&output.stderr));

    // 6. Verify Store for LFS tracking
    let mut gitattributes_path = None;
    for entry in walkdir::WalkDir::new(home_dir.path()) {
        let entry = entry.unwrap();
        if entry.file_name() == ".gitattributes" {
            gitattributes_path = Some(entry.path().to_path_buf());
            break;
        }
    }
    
    let gitattributes_path = gitattributes_path.expect("Store .gitattributes should exist");
    let attributes = fs::read_to_string(gitattributes_path).unwrap();
    
    assert!(attributes.contains(large_file), "Large file should be tracked in .gitattributes");
    assert!(attributes.contains("filter=lfs"), "LFS filter should be active");
    assert!(!attributes.contains(small_file), "Small file should NOT be tracked in LFS");
}
