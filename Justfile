#!/usr/bin/env just --justfile

# List every recipe.
default:
    @just --list --unsorted

# Format the workspace: nightly rustfmt, sorts and groups imports.
fmt:
    cargo +nightly fmt --all

# Run tests with nextest.
test *args:
    cargo nextest run --workspace --all-features --no-tests=pass {{ args }}

# Run tests under cargo-llvm-cov, write target/lcov.info, and print the coverage table.
# Usage: `just coverage` (whole workspace) or `just coverage my-crate` (single crate)
coverage crate="":
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -n "{{ crate }}" ]; then
        TARGET_ARGS="-p {{ crate }}"
    else
        TARGET_ARGS="--workspace"
    fi
    cargo llvm-cov nextest $TARGET_ARGS --all-features --lcov --output-path target/lcov.info --no-tests=pass
    cargo llvm-cov report

# Lint with clippy.
clippy *args:
    cargo clippy --workspace --all-targets --all-features {{ args }}
