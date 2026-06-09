#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_DIR="$ROOT/dist/MacCommandBar.app"
CONTENTS_DIR="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"

cargo build --manifest-path "$ROOT/core/Cargo.toml" --release
swift build --package-path "$ROOT" -c release

rm -rf "$APP_DIR"
mkdir -p "$MACOS_DIR" "$RESOURCES_DIR"

cp "$ROOT/.build/release/MacCommandBar" "$MACOS_DIR/MacCommandBar"
cp "$ROOT/core/target/release/mcb-core" "$RESOURCES_DIR/mcb-core"

cat > "$CONTENTS_DIR/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key>
  <string>MacCommandBar</string>
  <key>CFBundleIdentifier</key>
  <string>dev.blackcolours.MacCommandBar</string>
  <key>CFBundleName</key>
  <string>MacCommandBar</string>
  <key>CFBundleDisplayName</key>
  <string>MacCommandBar</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>0.1.0</string>
  <key>CFBundleVersion</key>
  <string>1</string>
  <key>LSMinimumSystemVersion</key>
  <string>14.0</string>
  <key>LSUIElement</key>
  <true/>
  <key>NSPrincipalClass</key>
  <string>NSApplication</string>
</dict>
</plist>
PLIST

chmod +x "$MACOS_DIR/MacCommandBar" "$RESOURCES_DIR/mcb-core"
codesign --force --deep --sign - "$APP_DIR" >/dev/null

echo "Built $APP_DIR"

