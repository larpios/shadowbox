use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs::{self, OpenOptions};
use std::io::Write;

pub fn get_repo_id() -> std::io::Result<String> {
    // Try to get remote origin URL
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output();
        
    match output {
        Ok(out) if out.status.success() => {
            let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
            // Normalize URL: remove git@, https://, and .git suffix
            let id = url.replace("https://", "")
                        .replace("git@", "")
                        .replace(":", "/")
                        .replace(".git", "");
            Ok(id)
        }
        _ => {
            // Fallback to absolute path
            let path = std::env::current_dir()?;
            Ok(path.to_string_lossy().to_string())
        }
    }
}

pub fn install_hooks() -> std::io::Result<()> {
    let hooks_dir = Path::new(".git/hooks");
    if !hooks_dir.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            ".git directory not found. Are you in a git repository?",
        ));
    }

    let hooks = ["post-checkout", "post-merge"];
    for hook_name in &hooks {
        let hook_path = hooks_dir.join(hook_name);
        
        if !hook_path.exists() {
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .open(&hook_path)?;
            writeln!(file, "#!/bin/sh")?;
        }

        // Use 'pull' instead of 'sync' to automatically fetch files
        append_if_missing(hook_path.to_str().expect("Valid path"), "shadowbox pull")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(&hook_path)?;
            let mut perms = metadata.permissions();
            perms.set_mode(0o755);
            let _ = std::fs::set_permissions(&hook_path, perms);
        }
    }
    Ok(())
}

pub fn uninstall_hooks() -> std::io::Result<()> {
    let hooks_dir = Path::new(".git/hooks");
    if !hooks_dir.exists() {
        return Ok(());
    }

    let hooks = ["post-checkout", "post-merge"];
    for hook_name in &hooks {
        let hook_path = hooks_dir.join(hook_name);
        if hook_path.exists() {
            remove_line(hook_path.to_str().expect("Valid path"), "shadowbox pull")?;
            remove_line(hook_path.to_str().expect("Valid path"), "shadowbox sync")?;
        }
    }
    Ok(())
}

pub fn append_if_missing(file_path: &str, line: &str) -> std::io::Result<()> {
    let contents = fs::read_to_string(file_path).unwrap_or_default();
    if !contents.lines().any(|l| l == line) {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)?;

        if !contents.is_empty() && !contents.ends_with('\n') {
            writeln!(file)?;
        }
        writeln!(file, "{}", line)?;
    }
    Ok(())
}

pub fn remove_line(file_path: &str, line_to_remove: &str) -> std::io::Result<()> {
    if let Ok(contents) = fs::read_to_string(file_path) {
        let lines: Vec<&str> = contents.lines().filter(|l| *l != line_to_remove).collect();
        if lines.len() < contents.lines().count() {
            let mut file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(file_path)?;

            for line in lines {
                writeln!(file, "{}", line)?;
            }
        }
    }
    Ok(())
}
