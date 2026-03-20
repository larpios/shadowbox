# Plan: Shadowbox

1. [x] Step 1 - Scaffold runnable Rust CLI entry point [checkpoint: 3ed2c19]
   - Demo: `cargo run -- --help` prints a basic command structure.
2. [x] Step 2 - Implement Core "Add/Track" Logic [checkpoint: d90b696]
   - Demo: `shadowbox track <file>` adds a file to the `.shadowbox` tracking file and updates `.gitignore` (and others).
3. [ ] Step 3 - Implement "Untrack" and "Status" Logic
   - Demo: `shadowbox untrack <file>` removes from tracking; `shadowbox status` lists managed files.
4. [ ] Step 4 - Implement Auto-setup Logic (Hooks)
   - Demo: `shadowbox init` sets up git/jj hooks to ensure the local setup persists.
5. [ ] Step 5 - Cross-platform and Verification
   - Demo: Ensure it runs on macOS (current) and passes basic cross-compilation/platform-agnostic tests.
