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
# Remove only the previous bundle: hashed game-client files, the page and
# the music folder (fetched by the game after it starts).
rm -f "$dest"/game-client-*.js "$dest"/game-client-*_bg.wasm "$dest"/index.html
rm -rf "$dest"/music
cp "$build"/index.html "$build"/game-client-*.js "$build"/game-client-*_bg.wasm "$dest"/
mkdir -p "$dest"/music
cp "$build"/music/*.ogg "$build"/music/manifest.json "$dest"/music/
echo "Exported to $dest:"
ls -lh "$dest"
