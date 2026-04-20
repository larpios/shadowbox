use std::path::PathBuf;
use std::process::Command;

pub fn get_repo_root() -> std::io::Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    if output.status.success() {
        let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(PathBuf::from(path_str))
    } else {
        std::env::current_dir()
    }
}

pub fn get_repo_id() -> std::io::Result<String> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output();
    match output {
        Ok(out) if out.status.success() => {
            let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
            Ok(url
                .replace("https://", "")
                .replace("git@", "")
                .replace(":", "/")
                .replace(".git", ""))
        }
        _ => {
            let path = std::env::current_dir()?;
            Ok(path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "default".to_string()))
        }
    }
}

pub fn install_hooks() -> std::io::Result<()> {
    let repo_root = get_repo_root()?;
    let hooks_dir = repo_root.join(".git").join("hooks");

    if !hooks_dir.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "Git hooks directory not found at {}. Is this a Git repository?",
                hooks_dir.display()
            ),
        ));
    }

    let hooks = ["post-checkout", "post-merge"];
    for hook_name in hooks {
        let hook_path = hooks_dir.join(hook_name);
        let mut content = if hook_path.exists() {
            std::fs::read_to_string(&hook_path)?
        } else {
            "#!/bin/sh\n".to_string()
        };

        if !content.contains("shadowbox pull") {
            if !content.ends_with('\n') && !content.is_empty() {
                content.push('\n');
            }
            content.push_str("shadowbox pull\n");
            std::fs::write(&hook_path, &content)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let metadata = std::fs::metadata(&hook_path)?;
                let mut perms = metadata.permissions();
                perms.set_mode(perms.mode() | 0o111);
                std::fs::set_permissions(&hook_path, perms)?;
            }
        }
    }

    Ok(())
}

pub fn uninstall_hooks() -> std::io::Result<()> {
    let repo_root = get_repo_root()?;
    let hooks_dir = repo_root.join(".git").join("hooks");

    if !hooks_dir.exists() {
        return Ok(());
    }

    let hooks = ["post-checkout", "post-merge"];
    for hook_name in hooks {
        let hook_path = hooks_dir.join(hook_name);
        if hook_path.exists() {
            let content = std::fs::read_to_string(&hook_path)?;
            if content.contains("shadowbox pull") {
                let lines: Vec<&str> = content.lines().collect();
                let filtered_lines: Vec<&str> = lines
                    .iter()
                    .cloned()
                    .filter(|line| !line.contains("shadowbox pull"))
                    .collect();

                if filtered_lines.is_empty()
                    || (filtered_lines.len() == 1 && filtered_lines[0] == "#!/bin/sh")
                {
                    std::fs::remove_file(&hook_path)?;
                } else {
                    let mut new_content = filtered_lines.join("\n");
                    if !new_content.is_empty() {
                        new_content.push('\n');
                    }
                    std::fs::write(&hook_path, new_content)?;
                }
            }
        }
    }

    Ok(())
}
