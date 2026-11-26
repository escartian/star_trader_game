#!/usr/bin/env bash
# Builds the browser version (release, relative paths) and copies it into a
# static site, e.g. a portfolio: tools/export_web.sh ~/Code/Resume/games/star-trader
# The destination's previous bundle is replaced; nothing else there is touched.
set -euo pipefail

dest="${1:?usage: tools/export_web.sh DEST_DIR}"
root="$(cd "$(dirname "$0")/.." && pwd)"
build="$(mktemp -d)"
trap 'rm -rf "$build"' EXIT

(cd "$root/crates/game-client" && trunk build --release --public-url ./ --dist "$build")

mkdir -p "$dest"
# Remove only the previous bundle: hashed game-client files and the page.
rm -f "$dest"/game-client-*.js "$dest"/game-client-*_bg.wasm "$dest"/index.html
cp "$build"/index.html "$build"/game-client-*.js "$build"/game-client-*_bg.wasm "$dest"/
echo "Exported to $dest:"
ls -lh "$dest"
