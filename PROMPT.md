# Shadowbox

Shadowbox is a CLI tool for managing local files you want to avoid committing to your repository.

My initial motivation for this was to avoid commiting a lot of the artifacts from different AI tools, but I realized it doesn't have to be limited to AI tools. It's a general purpose file manager for local files.
One example is a local justfile.

Your goal is to make this project possible.

## Specs

- If possible, I want this to be cross-platform
- If possible, I want this to happen automatically when you clone the repo you set up, through any of the following: git, jj, gh, etc.
- It should be architecturally accurate, and performant.

## Requirements

- Make atomic commits once you finish a task
- Be meticulous and accurate
- Do not use fragile workarounds. If you have to because it depends on some other significant task to be done, schedule it.
