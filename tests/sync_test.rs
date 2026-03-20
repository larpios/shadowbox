use std::fs;
use std::process::Command;

#[test]
fn test_sync_adds_missing_to_gitignore() {
    let shadowbox = ".shadowbox_sync";
    let gitignore = ".gitignore_sync";
    
    // Cleanup
    let _ = fs::remove_file(shadowbox);
    let _ = fs::remove_file(gitignore);

    // 1. Setup .shadowbox with file1
    fs::write(shadowbox, "file1\n").expect("Failed to write to .shadowbox");
    
    // 2. Setup empty .gitignore
    fs::write(gitignore, "").expect("Failed to write to .gitignore");

    // 3. Run sync
    let output = Command::new("cargo")
        .args(["run", "--", "sync"])
        .env("SHADOWBOX_FILE", shadowbox)
        .env("GITIGNORE_FILE", gitignore)
        .output()
        .expect("Failed to execute command");

    if !output.status.success() {
        panic!("Sync failed");
    }

    // 4. Verify .gitignore now contains file1
    let gitignore_content = fs::read_to_string(gitignore).expect("Failed to read .gitignore");
    let contains_file1 = gitignore_content.lines().any(|l| l == "file1");
    
    // Clean up
    let _ = fs::remove_file(shadowbox);
    let _ = fs::remove_file(gitignore);

    if !contains_file1 {
        panic!("file1 missing");
    }
}
