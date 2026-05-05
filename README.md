# Shadowbox 📦

**Shadowbox** is a cross-platform CLI tool for managing private files (like `.env` files, AI artifacts, local `justfiles`, or large binaries) that you want to keep in your workspace but avoid committing to your public repository.

Think of it like **chezmoi**, but specifically designed to manage private overlays for any number of unrelated public repositories without ever polluting them with metadata.

## Key Features

- 🛡️ **Zero Pollution:** No `.shadowbox` files or tracking receipts are ever created in your public project. All tracking metadata lives in your private store.
- 🔄 **Distributed Sync:** Pushes your private files to a secure, private Git repository and pulls them automatically when you're on a different machine.
- 🚀 **Auto-LFS:** Large files (over 5MB) are automatically detected and tracked via Git LFS in your private store—no manual configuration required.
- 🤖 **Seamless Automation:** Integrates with Git hooks (`post-checkout`, `post-merge`) to automatically restore your private environment.
- 🧩 **Smart Defaults:** If you use one central vault for all your projects, Shadowbox works out-of-the-box without needing manual project mapping.

## Installation

### Binary Releases
Pre-compiled binaries for Linux, macOS, and Windows are available on the [Releases page](https://github.com/youruser/shadowbox/releases).

### From crates.io
```bash
cargo install shadowbox
```

### From Source
```bash
cargo build --release
cp target/release/shadowbox /usr/local/bin/ # Or any directory in your PATH
```

## Getting Started (One-time Setup)

1. **Create a private Git repository** on GitHub, GitLab, or your own server to act as your "Shadowbox Store."
2. **Register your store:**
   ```bash
   shadowbox store add personal git@github.com:youruser/my-private-store.git
   ```
   *Shadowbox will automatically use this as your default store for all projects.*

## Usage

### 1. Initialize a Project
Run this once in a repository you want to protect:
```bash
shadowbox init
shadowbox hooks install
```

### 2. Track Private Files
Add files or entire directories. Shadowbox will add them to your `.gitignore` and start managing them in your private store.
```bash
shadowbox track .env
shadowbox track "configs/*.yaml"
shadowbox track models/  # Supports recursive directory tracking
```

### 3. Sync Your Changes
When you've added or modified private files:
```bash
shadowbox push
```

### 4. Restore on Another Machine
After cloning a public repo on a new machine:
```bash
shadowbox pull
```
*Note: If hooks are installed, future `git pull` or `git checkout` commands will automatically trigger a `shadowbox pull` for you.*

## Advanced Configuration

### Multiple Stores
If you have different vaults for work and personal projects:
```bash
shadowbox store add work git@github.com:work/vault.git
shadowbox map "github.com/work-org/*" work
shadowbox map "github.com/personal-user/*" personal
```

### Status
Check which files are currently being tracked for the current project:
```bash
shadowbox status
```

## How it Works

Shadowbox maintains local clones of your private stores in your system's standard data directory (e.g., `~/.local/share/shadowbox` on Linux or `~/Library/Application Support/com.shadowbox.shadowbox` on macOS). 

When you `push`, Shadowbox organizes your files within that store using a unique identifier derived from your project's remote origin URL (e.g., `github.com/user/project/`). This ensures that one private vault can safely manage hundreds of different projects without collisions.

## Requirements

- Git
- Git LFS (Optional, recommended for large files)

## License
Apache
