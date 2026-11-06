#!/usr/bin/env bash
# Build target/StarTrader.app so macOS shows the Blender-rendered app icon in the
# Dock and Finder. (macOS ignores runtime window icons; Windows/X11 use the one
# embedded in the binary.) Unsigned: for local play, not distribution.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo build --release -p game-client
APP=target/StarTrader.app
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/game-client "$APP/Contents/Resources/game-client"
cp -R config "$APP/Contents/Resources/config"

# Icon set from the 1024px render; iconutil ships with macOS.
ICONSET=$(mktemp -d)/AppIcon.iconset
mkdir -p "$ICONSET"
for size in 16 32 128 256 512; do
  sips -z $size $size assets/app-icon/app-icon.png --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
  double=$((size * 2))
  sips -z $double $double assets/app-icon/app-icon.png --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/AppIcon.icns"

# Saves and settings use relative paths, so run from a per-user data folder.
cat > "$APP/Contents/MacOS/StarTrader" <<'LAUNCHER'
#!/bin/bash
RESOURCES="$(cd "$(dirname "$0")/../Resources" && pwd)"
DATA="$HOME/Library/Application Support/StarTrader"
mkdir -p "$DATA"
[ -d "$DATA/config" ] || cp -R "$RESOURCES/config" "$DATA/config"
cd "$DATA" && exec "$RESOURCES/game-client" "$@"
LAUNCHER
chmod +x "$APP/Contents/MacOS/StarTrader"

cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Star Trader</string>
  <key>CFBundleDisplayName</key><string>Star Trader</string>
  <key>CFBundleIdentifier</key><string>dev.escartian.startrader</string>
  <key>CFBundleExecutable</key><string>StarTrader</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST
echo "Built $APP — open it with: open $APP"
