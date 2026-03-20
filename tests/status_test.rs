use std::fs::{self, File};
use std::io::Write;
use std::process::Command;

#[test]
fn test_status_command() {
    let shadowbox_path = ".shadowbox_test_status";
    let mut file = File::create(shadowbox_path).unwrap();
    writeln!(file, "file1.txt").unwrap();
    writeln!(file, "dir/file2.txt").unwrap();

    // Create the files so they aren't marked as missing for this test
    File::create("file1.txt").unwrap();
    fs::create_dir_all("dir").unwrap();
    File::create("dir/file2.txt").unwrap();

    let output = Command::new("cargo")
        .args(["run", "--", "status"])
        .env("SHADOWBOX_FILE", shadowbox_path)
        .output()
        .expect("Failed to execute command");

    fs::remove_file(shadowbox_path).ok();
    fs::remove_file("file1.txt").ok();
    fs::remove_dir_all("dir").ok();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("file1.txt"));
    assert!(stdout.contains("dir/file2.txt"));
}

#[test]
fn test_status_missing_file() {
    let shadowbox_path = ".shadowbox_test_missing";
    let mut file = File::create(shadowbox_path).unwrap();
    writeln!(file, "missing_file.txt").expect("Failed to write to shadowbox file");
    writeln!(file, "existing_file.txt").expect("Failed to write to shadowbox file");

    // Create the existing file
    File::create("existing_file.txt").expect("Failed to create existing file");

    let output = Command::new("cargo")
        .args(["run", "--", "status"])
        .env("SHADOWBOX_FILE", shadowbox_path)
        .output()
        .expect("Failed to execute command");

    // Clean up
    fs::remove_file(shadowbox_path).ok();
    fs::remove_file("existing_file.txt").ok();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[MISSING] missing_file.txt"),
        "Output should contain [MISSING] missing_file.txt, but was:\n{}",
        stdout
    );
    assert!(
        stdout.contains("existing_file.txt"),
        "Output should contain existing_file.txt"
    );
    assert!(
        !stdout.contains("[MISSING] existing_file.txt"),
        "Output should NOT contain [MISSING] existing_file.txt"
    );
}
