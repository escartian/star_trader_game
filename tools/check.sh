#!/bin/sh
# Run from any directory. Cargo uses the committed dependency lockfile.
set -eu
cd "$(dirname "$0")/.."
python3 tools/check_file_lengths.py
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
