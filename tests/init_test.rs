use std::process::Command;
use tempfile::tempdir;
use std::env;

#[test]
fn test_init_command() {
    let dir = tempdir().expect("Failed to create temp dir");
    // Use simple names because the env vars will override the default behavior in main.rs
    // But main.rs prints the string it gets from the env var or default.
    let shadowbox_file_name = ".shadowbox";
    let gitignore_file_name = ".gitignore";
    
    // Get the absolute path to the shadowbox binary
    let binary_path = env::current_dir().unwrap().join("target/debug/shadowbox");
    
    // Ensure binary is built
    let _ = Command::new("cargo")
        .arg("build")
        .status()
        .expect("Failed to build shadowbox");

    // Run 'shadowbox init' in the temp directory
    let output = Command::new(binary_path)
        .arg("init")
        .env("SHADOWBOX_FILE", shadowbox_file_name)
        .env("GITIGNORE_FILE", gitignore_file_name)
        .current_dir(dir.path())
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("STDOUT: {}", stdout);
    println!("STDERR: {}", stderr);

    assert!(output.status.success());
    assert!(stdout.contains("Initialized .shadowbox"));
    assert!(stdout.contains("Initialized .gitignore"));

    assert!(dir.path().join(shadowbox_file_name).exists());
    assert!(dir.path().join(gitignore_file_name).exists());
}
