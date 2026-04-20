use crate::config::Config;
use crate::git::{get_repo_id, get_repo_root};
use crate::utils::{get_vault_project_path, is_binary, resolve_store};
use colored::Colorize;
use std::fs;
use std::path::Path;

#[derive(Debug)]
struct FileStatus {
    path: String,
    state: String,
    file_type: String,
    target: Option<String>,
}

pub fn run() -> std::io::Result<()> {
    let config = Config::load()?;
    let repo_id = get_repo_id()?;
    let vault_project_path = get_vault_project_path(&config, &repo_id)?;
    let repo_root = get_repo_root()?;
    let store_name = resolve_store(&config, &repo_id)
        .map(|s| s.as_str())
        .unwrap_or("unknown");

    let mut results = Vec::new();
    if vault_project_path.exists() {
        collect_status(
            &vault_project_path,
            &vault_project_path,
            &repo_root,
            &mut results,
        )?;
    }

    if results.is_empty() {
        println!("{}", "No files tracked in this repository.".yellow());
        return Ok(());
    }

    let mut max_path_len = 40;
    let mut max_target_len = 30;

    for file in &results {
        if file.path.len() > max_path_len {
            max_path_len = file.path.len();
        }
        if let Some(target) = &file.target
            && target.len() > max_target_len
        {
            max_target_len = target.len();
        }
    }

    println!(
        "Status for repo: {} (Vault: {})",
        repo_id.bold(),
        store_name.cyan()
    );
    println!();

    let header_state = format!("{:<10}", "STATE");
    let header_type = format!("{:<10}", "TYPE");
    let header_path = format!("{:<width$}", "PATH", width = max_path_len);
    let header_target = format!("{:<width$}", "TARGET", width = max_target_len);

    println!(
        "| {} | {} | {} | {} |",
        header_state.bold(),
        header_type.bold(),
        header_path.bold(),
        header_target.bold()
    );

    println!(
        "|{}|{}|{}|{}|",
        "-".repeat(12),
        "-".repeat(12),
        "-".repeat(max_path_len + 2),
        "-".repeat(max_target_len + 2)
    );

    for file in results {
        let target_str = file.target.as_deref().unwrap_or("");

        let state_col = match file.state.as_str() {
            "TRACKED" => format!("{:<10}", file.state).green(),
            "MISSING" => format!("{:<10}", file.state).red(),
            _ => format!("{:<10}", file.state).normal(),
        };

        let type_col = match file.file_type.as_str() {
            "DIR" => format!("{:<10}", file.file_type).blue(),
            "SYMLINK" => format!("{:<10}", file.file_type).cyan(),
            "BINARY" => format!("{:<10}", file.file_type).magenta(),
            "FILE" => format!("{:<10}", file.file_type).normal(),
            _ => format!("{:<10}", file.file_type).normal(),
        };

        let path_col = format!("{:<width$}", file.path, width = max_path_len).normal();
        let target_col = format!("{:<width$}", target_str, width = max_target_len).bright_black();

        println!(
            "| {} | {} | {} | {} |",
            state_col, type_col, path_col, target_col
        );
    }
    Ok(())
}

fn collect_status(
    vault_root: &Path,
    current_dir: &Path,
    repo_root: &Path,
    results: &mut Vec<FileStatus>,
) -> std::io::Result<()> {
    let mut entries: Vec<_> = fs::read_dir(current_dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| e.path());

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name();
        if name == ".git" {
            continue;
        }
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.is_dir() {
            collect_status(vault_root, &path, repo_root, results)?;
        } else {
            let rel_path = path.strip_prefix(vault_root).unwrap();
            let local_path = repo_root.join(rel_path);
            let display_name = rel_path.to_string_lossy().to_string();

            let mut state = "TRACKED".to_string();
            let mut file_type = "FILE".to_string();
            let mut target = None;

            match fs::symlink_metadata(&local_path) {
                Ok(local_metadata) => {
                    if local_metadata.is_symlink() {
                        file_type = "SYMLINK".to_string();
                        if let Ok(link_target) = fs::read_link(&local_path) {
                            target = Some(format!("-> {}", link_target.display()));
                        }
                    } else if local_metadata.is_dir() {
                        file_type = "DIR".to_string();
                    } else if is_binary(&local_path).unwrap_or(false) {
                        file_type = "BINARY".to_string();
                    }
                }
                Err(_) => {
                    state = "MISSING".to_string();
                }
            }

            results.push(FileStatus {
                path: display_name,
                state,
                file_type,
                target,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use tempfile::tempdir;

    fn run_shadowbox(args: Vec<&str>, current_dir: &Path, home_dir: &Path) -> std::process::Output {
        let binary_path = env::current_dir().unwrap().join("target/debug/shadowbox");
        Command::new(&binary_path)
            .args(args)
            .current_dir(current_dir)
            .env("HOME", home_dir)
            .env("XDG_CONFIG_HOME", home_dir.join(".config"))
            .env("XDG_DATA_HOME", home_dir.join(".local/share"))
            .output()
            .expect("Failed to execute shadowbox")
    }

    #[test]
    fn test_status_command() {
        let home_dir = tempdir().expect("Failed to create temp home");
        let remote_store_dir = tempdir().expect("Failed to create temp remote store");
        let dir = tempdir().expect("Failed to create temp project");

        Command::new("git")
            .args(["init", "--bare"])
            .current_dir(remote_store_dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["remote", "add", "origin", "https://github.com/user/project"])
            .current_dir(dir.path())
            .status()
            .unwrap();

        run_shadowbox(
            vec![
                "store",
                "add",
                "my_store",
                &remote_store_dir.path().to_string_lossy(),
            ],
            dir.path(),
            home_dir.path(),
        );
        run_shadowbox(vec!["map", "**", "my_store"], dir.path(), home_dir.path());

        let file1 = "file1.txt";
        fs::write(dir.path().join(file1), "content").unwrap();
        run_shadowbox(vec!["track", file1], dir.path(), home_dir.path());

        let output = run_shadowbox(vec!["status"], dir.path(), home_dir.path());
        assert!(String::from_utf8_lossy(&output.stdout).contains(file1));
    }

    #[test]
    fn test_status_missing_file() {
        let home_dir = tempdir().expect("Failed to create temp home");
        let remote_store_dir = tempdir().expect("Failed to create temp remote store");
        let dir = tempdir().expect("Failed to create temp project");

        Command::new("git")
            .args(["init", "--bare"])
            .current_dir(remote_store_dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["remote", "add", "origin", "https://github.com/user/project"])
            .current_dir(dir.path())
            .status()
            .unwrap();

        run_shadowbox(
            vec![
                "store",
                "add",
                "my_store",
                &remote_store_dir.path().to_string_lossy(),
            ],
            dir.path(),
            home_dir.path(),
        );
        run_shadowbox(vec!["map", "**", "my_store"], dir.path(), home_dir.path());

        let file1 = "missing.txt";
        fs::write(dir.path().join(file1), "content").unwrap();
        run_shadowbox(vec!["track", file1], dir.path(), home_dir.path());
        fs::remove_file(dir.path().join(file1)).unwrap();

        let output = run_shadowbox(vec!["status"], dir.path(), home_dir.path());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("MISSING"));
        assert!(stdout.contains("missing.txt"));
    }
}
