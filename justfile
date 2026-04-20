# https://just.systems

default:
    @just -l

fmt:
    @echo "Running cargo fmt..."
    @cargo fmt
    @echo "Done"

fix:
    @echo "Running cargo fix --allow-dirty..."
    @cargo fix --allow-dirty
    @echo "Done"

clippy-fix:
    @echo "Running cargo clippy --fix --allow-dirty..."
    @cargo clippy --fix --allow-dirty
    @echo "Done"

fix-all: fix clippy-fix fmt 

test:
    @echo "Running cargo test --workspace..."
    @cargo test --workspace
    @echo "Test done"

check-format:
    @echo "Running cargo fmt --all --check..."
    @cargo fmt --all --check
    @echo "Format check done"

check-lint:
    @echo "Running cargo clippy --workspace --all-targets --all-features -- -D warnings..."
    @cargo clippy --workspace --all-targets --all-features -- -D warnings
    @echo "Lint check done"

check: check-format check-lint test
