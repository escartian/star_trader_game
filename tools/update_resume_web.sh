#!/usr/bin/env bash
# Build the web game and replace the copy in the sibling Resume repository.
# Run from anywhere: ./tools/update_resume_web.sh
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
games="$(cd "$root/../Resume/games" && pwd)"
dest="$games/star-trader"
if [[ ! -d "$dest" || ! -f "$dest/index.html" ]]; then
    echo "Expected the existing Resume game at $dest" >&2
    exit 1
fi

# Build and validate in the same filesystem as the destination, then swap
# directories. If the second move fails, the old game is put back.
build="$(mktemp -d)"
staged="$(mktemp -d "$games/.star-trader-new.XXXXXX")"
backup="$(mktemp -d "$games/.star-trader-old.XXXXXX")"
rmdir "$backup"
cleanup() {
    if [[ -d "$backup" && ! -e "$dest" ]]; then
        mv "$backup" "$dest"
    fi
    rm -rf "$build"
    rm -rf "$staged"
}
trap cleanup EXIT

(cd "$root/crates/game-client" && trunk build --release --public-url ./ --dist "$build")
test -s "$build/index.html"
test -s "$build/music/manifest.json"
compgen -G "$build/game-client-*.js" >/dev/null
compgen -G "$build/game-client-*_bg.wasm" >/dev/null

# The new directory links unchanged files to the old one. `cmp` checks bytes,
# so a touched timestamp alone does not cause 33 MB of music to be recopied.
# The links stay valid after the old directory is removed during the swap.
reused=0
copied=0
while IFS= read -r -d '' file; do
    relative="${file#"$build"/}"
    mkdir -p "$(dirname "$staged/$relative")"
    if [[ -f "$dest/$relative" ]] && cmp -s "$file" "$dest/$relative"; then
        ln "$dest/$relative" "$staged/$relative"
        reused=$((reused + 1))
    else
        cp "$file" "$staged/$relative"
        copied=$((copied + 1))
    fi
done < <(find "$build" -type f -print0)

test -s "$staged/index.html"
test -s "$staged/music/manifest.json"

mv "$dest" "$backup"
mv "$staged" "$dest"
rm -rf "$backup"
echo "Updated $dest ($copied changed files copied, $reused unchanged files reused)"
