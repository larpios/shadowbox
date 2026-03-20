# Autoresearch: Shadowbox Initial Prototype

## Objective
Implement a functional MVP of Shadowbox that can sync local files to a storage backend (S3-compatible) with encryption. The primary focus is on the core "shadow" functionality: ignoring files in the main repo while managing them via Shadowbox.

## Metrics
- **Primary**: Successful Sync (bool, higher is better)
- **Current Best**: 1 (Successful Sync)
- **Secondary**: Sync speed (ms), Encryption overhead (%), cross-platform compatibility (verified OS count)

## Benchmark Command
`cargo run -- sync --test-mode`
Parse success/failure and timing from stdout.

## Files in Scope
- `Cargo.toml`: Project dependencies.
- `src/main.rs`: Entry point.
- `src/config.rs`: Configuration handling.
- `src/sync.rs`: Sync logic.
- `src/crypto.rs`: Encryption logic.
- `src/storage/`: Storage backends.

## Off Limits
- `.git/`: Must not modify git internals directly (except via hooks if needed).

## Constraints
- Rust must be the primary language for the core.
- Tests must pass.
- Encryption is mandatory.
- Cross-platform support (Linux, macOS, Windows).

## What's Been Tried
- **Run 1 (DISCARD, metric=1)**: Implement basic configuration management (config-init). Hypothesis: Configuration management is required for real-world usage. Result: DISCARD because metric stayed at 1 but complexity and duration increased. Rule: Discard when metric is equal and code was added.
- Initial project scoping and branch creation.
- **Run 0 (KEEP, metric=1)**: Baseline: Initial project setup and core sync logic verification. Hypothesis: Base implementation is functional.
