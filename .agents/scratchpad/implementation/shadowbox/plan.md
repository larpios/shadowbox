# Plan - Shadowbox Implementation

1. Step 1: Core Robustification and Edge-Case Handling
   - Goal: Address known bugs in path normalization and harden the track/untrack logic.
   - Demo: `track .` is rejected or handled safely, and complex paths are normalized correctly.
2. Step 2: Storage Layer Refinement and Status Features
   - Goal: Ensure the storage format is robust and `status` provides clear, useful information about tracked files.
   - Demo: `status` shows tracked files and identifies any that are missing on disk.
3. Step 3: Automatic Integration Hooks
   - Goal: Implement the logic to automate `.gitignore` updates and hook into repository life-cycles.
   - Demo: Shadowbox can sync `.gitignore` based on `.shadowbox`, and install git hooks for automatic sync.
