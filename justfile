# Riffl Automation - https://just.systems
# All commands are implemented in Rust within the 'xtask' crate.

# List all available commands (Default)
default:
    @cargo xtask --help

# Launch the Riffl TUI music tracker
run *args:
    @cargo xtask run -- {{args}}

# Format the entire codebase
fmt:
    @cargo xtask fmt

# Run all CI checks (formatting, clippy, tests)
check:
    @cargo xtask check

# Run the test suite
test *args:
    @cargo xtask test -- {{args}}

# Increment version: patch, minor, or major (e.g. just bump patch)
bump part:
    @cargo xtask bump {{part}}

# Set a specific version (e.g. just bump-to 0.2.0)
bump-to version:
    @cargo xtask bump-to {{version}}
