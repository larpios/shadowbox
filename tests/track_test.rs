use std::fs;
use std::process::Command;

#[test]
fn test_track_subcommand() {
    let test_file = "test_track_file_1.txt";
    let shadowbox = ".shadowbox_1";
    let gitignore = ".gitignore_1";
    fs::write(test_file, "content").expect("Failed to write test file");

    let output = Command::new("cargo")
        .args(["run", "--", "track", "./test_track_file_1.txt"])
        .env("SHADOWBOX_FILE", shadowbox)
        .env("GITIGNORE_FILE", gitignore)
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Tracked test_track_file_1.txt"));

    let shadowbox_content = fs::read_to_string(shadowbox).unwrap_or_default();
    assert!(
        shadowbox_content
            .lines()
            .any(|l| l == "test_track_file_1.txt")
    );

    let output2 = Command::new("cargo")
        .args(["run", "--", "track", "test_track_file_1.txt"])
        .env("SHADOWBOX_FILE", shadowbox)
        .env("GITIGNORE_FILE", gitignore)
        .output()
        .expect("Failed to execute command");

    assert!(output2.status.success());

    let shadowbox_content2 = fs::read_to_string(shadowbox).unwrap_or_default();
    let count = shadowbox_content2
        .lines()
        .filter(|l| l == &"test_track_file_1.txt")
        .count();
    assert_eq!(count, 1, "Should only have one entry for the same file");

    fs::remove_file(test_file).ok();
    fs::remove_file(shadowbox).ok();
    fs::remove_file(gitignore).ok();
}

#[test]
fn test_path_normalization() {
    let subdir = "test_subdir_norm_2";
    let test_file = "test_subdir_norm_2/test_file.txt";
    let shadowbox = ".shadowbox_2";
    let gitignore = ".gitignore_2";
    fs::create_dir_all(subdir).expect("Failed to create subdir");
    fs::write(test_file, "content").expect("Failed to write test file");

    let complex_path = "./test_subdir_norm_2/../test_subdir_norm_2/./test_file.txt";
    let output = Command::new("cargo")
        .args(["run", "--", "track", complex_path])
        .env("SHADOWBOX_FILE", shadowbox)
        .env("GITIGNORE_FILE", gitignore)
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Tracked test_subdir_norm_2/test_file.txt"));

    let shadowbox_content = fs::read_to_string(shadowbox).unwrap_or_default();
    assert!(
        shadowbox_content
            .lines()
            .any(|l| l == "test_subdir_norm_2/test_file.txt")
    );

    fs::remove_dir_all(subdir).ok();
    fs::remove_file(shadowbox).ok();
    fs::remove_file(gitignore).ok();
}

#[test]
fn test_path_normalization_edge_cases() {
    let test_file = "edge_case.txt";
    let shadowbox = ".shadowbox_edge";
    let gitignore = ".gitignore_edge";
    fs::write(test_file, "content").expect("Failed to write test file");

    // Test "." normalization
    let output = Command::new("cargo")
        .args(["run", "--", "track", "."])
        .env("SHADOWBOX_FILE", shadowbox)
        .env("GITIGNORE_FILE", gitignore)
        .output()
        .expect("Failed to execute command");

    // "." is a directory, not a file, and track_file checks for raw_path.exists()
    // In track_file: if !raw_path.exists() { return Err ... }
    // "." exists, so it should proceed to normalize.

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Tracked ."));

    let content = fs::read_to_string(shadowbox).unwrap_or_default();
    assert!(
        content.lines().any(|l| l == "."),
        "Should have '.' in shadowbox"
    );

    // Test ".." normalization from within a subdir
    let subdir = "test_edge_subdir";
    fs::create_dir_all(subdir).expect("Failed to create subdir");

    let complex_path = "test_edge_subdir/../edge_case.txt";
    let output2 = Command::new("cargo")
        .args(["run", "--", "track", complex_path])
        .env("SHADOWBOX_FILE", shadowbox)
        .env("GITIGNORE_FILE", gitignore)
        .output()
        .expect("Failed to execute command");

    assert!(output2.status.success());
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    assert!(stdout2.contains("Tracked edge_case.txt"));

    fs::remove_file(test_file).ok();
    fs::remove_dir_all(subdir).ok();
    fs::remove_file(shadowbox).ok();
    fs::remove_file(gitignore).ok();
}

#[test]
fn test_append_without_newline() {
    let test_file = "test_file_bug_3.txt";
    let shadowbox = ".shadowbox_3";
    let gitignore = ".gitignore_3";
    fs::write(test_file, "content").expect("Failed to write test file");

    fs::write(shadowbox, "initial_line").expect("Failed to write initial line");

    let output = Command::new("cargo")
        .args(["run", "--", "track", test_file])
        .env("SHADOWBOX_FILE", shadowbox)
        .env("GITIGNORE_FILE", gitignore)
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let content = fs::read_to_string(shadowbox).expect("Failed to read shadowbox");
    // Check if fixed: content will contain "initial_line\ntest_file_bug_3.txt\n"
    assert!(
        content.contains("\ninitial_line\n") || content.starts_with("initial_line\n"),
        "Should have newline after initial_line, got: {:?}",
        content
    );
    assert!(
        content.contains("test_file_bug_3.txt\n"),
        "Should have track line, got: {:?}",
        content
    );

    fs::remove_file(test_file).ok();
    fs::remove_file(shadowbox).ok();
    fs::remove_file(gitignore).ok();
}
