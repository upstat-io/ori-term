#!/bin/bash
# Build oriterm as a macOS .app bundle.
#
# Usage:
#   ./bundle-macos.sh          # debug build
#   ./bundle-macos.sh release  # release build
#
# Output: target/<profile>/Oriterm.app

set -euo pipefail

PROFILE="${1:-debug}"
if [ "$PROFILE" = "release" ]; then
    cargo build --release --bin oriterm --bin oriterm-mux
    TARGET_DIR="target/release"
else
    cargo build --bin oriterm --bin oriterm-mux
    TARGET_DIR="target/debug"
fi

APP_DIR="$TARGET_DIR/Oriterm.app"
CONTENTS="$APP_DIR/Contents"
MACOS="$CONTENTS/MacOS"
RESOURCES="$CONTENTS/Resources"

# Clean previous bundle.
rm -rf "$APP_DIR"
mkdir -p "$MACOS" "$RESOURCES"

# Copy binaries.
cp "$TARGET_DIR/oriterm" "$MACOS/oriterm"
cp "$TARGET_DIR/oriterm-mux" "$MACOS/oriterm-mux"

# Generate .icns from PNG assets using sips + iconutil.
ICONSET_DIR=$(mktemp -d)/oriterm.iconset
mkdir -p "$ICONSET_DIR"
cp assets/icon-16.png "$ICONSET_DIR/icon_16x16.png"
cp assets/icon-32.png "$ICONSET_DIR/icon_16x16@2x.png"
cp assets/icon-32.png "$ICONSET_DIR/icon_32x32.png"
cp assets/icon-64.png "$ICONSET_DIR/icon_32x32@2x.png"
cp assets/icon-128.png "$ICONSET_DIR/icon_128x128.png"
cp assets/icon-256.png "$ICONSET_DIR/icon_128x128@2x.png"
cp assets/icon-256.png "$ICONSET_DIR/icon_256x256.png"
# No 512px source — reuse 256 for 256@2x.
cp assets/icon-256.png "$ICONSET_DIR/icon_256x256@2x.png"
iconutil -c icns "$ICONSET_DIR" -o "$RESOURCES/oriterm.icns" 2>/dev/null || {
    echo "warning: iconutil failed, app will have no icon"
}
rm -rf "$(dirname "$ICONSET_DIR")"

# Write Info.plist.
cat > "$CONTENTS/Info.plist" << 'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>Oriterm</string>
    <key>CFBundleDisplayName</key>
    <string>Oriterm</string>
    <key>CFBundleIdentifier</key>
    <string>com.oriterm.app</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundleExecutable</key>
    <string>oriterm</string>
    <key>CFBundleIconFile</key>
    <string>oriterm</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSMinimumSystemVersion</key>
    <string>13.0</string>
</dict>
</plist>
PLIST

echo "Built: $APP_DIR"
echo ""
echo "To install:  cp -r $APP_DIR /Applications/"
echo "To run:      open $APP_DIR"
