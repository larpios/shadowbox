use crate::git;

pub fn install() -> std::io::Result<()> {
    git::install_hooks()?;
    println!("Hooks installed successfully.");
    Ok(())
}

pub fn uninstall() -> std::io::Result<()> {
    git::uninstall_hooks()?;
    println!("Hooks uninstalled successfully.");
    Ok(())
}

#[cfg(test)]
mod tests {
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

        let output = Command::new(&binary_path)
            .arg("hooks")
            .arg("install")
            .current_dir(repo_root)
            .output()
            .expect("Failed to execute command");

        assert!(output.status.success());

        for hook_name in &["post-checkout", "post-merge"] {
            let hook_path = git_hooks_dir.join(hook_name);
            assert!(hook_path.exists());
            let mut content = String::new();
            fs::File::open(&hook_path)
                .unwrap()
                .read_to_string(&mut content)
                .unwrap();
            assert!(content.contains("shadowbox pull"));

            #[cfg(unix)]
            {
                let metadata = fs::metadata(&hook_path).unwrap();
                assert!(metadata.permissions().mode() & 0o111 != 0);
            }
        }

        let output = Command::new(&binary_path)
            .arg("hooks")
            .arg("uninstall")
            .current_dir(repo_root)
            .output()
            .expect("Failed to execute command");

        assert!(output.status.success());
        for hook_name in &["post-checkout", "post-merge"] {
            let hook_path = git_hooks_dir.join(hook_name);
            if hook_path.exists() {
                let mut content = String::new();
                fs::File::open(&hook_path)
                    .unwrap()
                    .read_to_string(&mut content)
                    .unwrap();
                assert!(!content.contains("shadowbox pull"));
            }
        }
    }
}
