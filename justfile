# Justfile for Riffl - https://just.systems

# Default: List available recipes
default:
    @just --list

# Format all crates in the workspace
fmt:
    cargo fmt --all

# Run full CI-style checks (formatting, clippy, tests)
check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-features -- -D warnings
    cargo test --workspace --all-features

# Run the 'riffl' TUI application
run *args:
    cargo run --bin riffl {{args}}

# Run all tests in the workspace
test *args:
    cargo test --workspace {{args}}

# Standard git workflow (use with caution if using jj)
fmt-and-commit:
    cargo fmt --all 
    git add .
    git commit -m "chore(fmt): format code"
