#!/bin/sh
# Run from any directory. Cargo uses the committed dependency lockfile.
set -eu
cd "$(dirname "$0")/.."
python3 tools/check_file_lengths.py
cargo fmt --all --check
# Lint both the player build (no dev tooling) and the developer build.
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
