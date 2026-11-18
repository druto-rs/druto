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

# Run tests under cargo-llvm-cov and write target/lcov.info.
coverage:
    cargo llvm-cov nextest --workspace --all-features --lcov --output-path target/lcov.info --summary-only --no-tests=pass

# Lint with clippy.
clippy *args:
    cargo clippy --workspace --all-targets --all-features {{ args }}
