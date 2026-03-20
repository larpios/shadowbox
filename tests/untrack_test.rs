use std::fs;
use std::process::Command;

#[test]
fn test_untrack_subcommand() {
    let test_file = "test_untrack_file.txt";
    let shadowbox = ".shadowbox_untrack";
    let gitignore = ".gitignore_untrack";

    // 0. Cleanup from previous runs
    let _ = fs::remove_file(test_file);
    let _ = fs::remove_file(shadowbox);
    let _ = fs::remove_file(gitignore);

    // 1. Setup: Create and Track a file
    fs::write(test_file, "content").expect("Failed to write test file");

    // First track it
    let track_output = Command::new("cargo")
        .args(["run", "--", "track", test_file])
        .env("SHADOWBOX_FILE", shadowbox)
        .env("GITIGNORE_FILE", gitignore)
        .output()
        .expect("Failed to execute track command");

    assert!(track_output.status.success());

    // Verify it IS tracked
    let shadowbox_content = fs::read_to_string(shadowbox).expect("Failed to read shadowbox");
    assert!(
        shadowbox_content.lines().any(|l| l == test_file),
        "Should be tracked initially"
    );

    // 2. Act: Untrack the file
    let output_untrack = Command::new("cargo")
        .args(["run", "--", "untrack", test_file])
        .env("SHADOWBOX_FILE", shadowbox)
        .env("GITIGNORE_FILE", gitignore)
        .output()
        .expect("Failed to execute untrack command");

    assert!(output_untrack.status.success());

    // 3. Assert
    let shadowbox_after = fs::read_to_string(shadowbox).unwrap_or_default();
    assert!(
        !shadowbox_after.lines().any(|l| l == test_file),
        "Should NOT be tracked after untrack"
    );

    let gitignore_after = fs::read_to_string(gitignore).unwrap_or_default();
    assert!(
        !gitignore_after.lines().any(|l| l == test_file),
        "Should NOT be in gitignore after untrack"
    );

    // Cleanup
    let _ = fs::remove_file(test_file);
    let _ = fs::remove_file(shadowbox);
    let _ = fs::remove_file(gitignore);
}

#[test]
fn test_untrack_normalization() {
    let test_file = "test_untrack_norm.txt";
    let shadowbox = ".shadowbox_norm_untrack";
    let gitignore = ".gitignore_norm_untrack";

    fs::write(test_file, "content").expect("Failed to write test file");

    // Track with raw path
    let _ = Command::new("cargo")
        .args(["run", "--", "track", test_file])
        .env("SHADOWBOX_FILE", shadowbox)
        .env("GITIGNORE_FILE", gitignore)
        .output();

    // Untrack with ./ path
    let output_untrack = Command::new("cargo")
        .args(["run", "--", "untrack", &format!("./{}", test_file)])
        .env("SHADOWBOX_FILE", shadowbox)
        .env("GITIGNORE_FILE", gitignore)
        .output()
        .expect("Failed to execute untrack command");

    assert!(output_untrack.status.success());

    let shadowbox_after = fs::read_to_string(shadowbox).unwrap_or_default();
    assert!(
        !shadowbox_after.lines().any(|l| l == test_file),
        "Should be untracked even if untrack path used ./ prefix"
    );

    fs::remove_file(test_file).ok();
    fs::remove_file(shadowbox).ok();
    fs::remove_file(gitignore).ok();
}

#[test]
fn test_untrack_dot_regression() {
    let shadowbox = ".shadowbox_dot_test";
    let gitignore = ".gitignore_dot_test";

    // Setup shadowbox with some content and a blank line
    let initial_content = "file1.txt\n\nfile2.txt\n";
    fs::write(shadowbox, initial_content).expect("Failed to write shadowbox");
    fs::write(gitignore, initial_content).expect("Failed to write gitignore");

    // Act: Untrack "."
    let output = Command::new("cargo")
        .args(["run", "--", "untrack", "."])
        .env("SHADOWBOX_FILE", shadowbox)
        .env("GITIGNORE_FILE", gitignore)
        .output()
        .expect("Failed to execute untrack command");

    // Even if it "succeeds" in normalization, it shouldn't have stripped everything
    // But based on the memory, it might return empty string and strip all blank lines.

    let content = fs::read_to_string(shadowbox).expect("Failed to read shadowbox");
    assert_eq!(
        content, initial_content,
        "Shadowbox content should not change when untracking '.'"
    );

    fs::remove_file(shadowbox).ok();
    fs::remove_file(gitignore).ok();
}
