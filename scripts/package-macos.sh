#!/bin/zsh
set -euo pipefail

repo_dir="${0:A:h:h}"
cd "$repo_dir"
cargo build --release

bundle="$repo_dir/dist/BetterDeck.app"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
cp "$repo_dir/target/release/betterdeck" "$bundle/Contents/MacOS/BetterDeck"

icon_source="$repo_dir/assets/betterdeck-app-icon.png"
iconset="$repo_dir/target/BetterDeck.iconset"
icon_square="$repo_dir/target/betterdeck-icon-square.png"
icon_width="$(sips -g pixelWidth "$icon_source" | awk '/pixelWidth:/ { print $2 }')"
icon_height="$(sips -g pixelHeight "$icon_source" | awk '/pixelHeight:/ { print $2 }')"
if (( icon_width < icon_height )); then
  icon_side="$icon_width"
else
  icon_side="$icon_height"
fi
mkdir -p "$iconset"
sips --cropToHeightWidth "$icon_side" "$icon_side" "$icon_source" --out "$icon_square" >/dev/null
for spec in \
  icon_16x16.png:16 icon_16x16@2x.png:32 \
  icon_32x32.png:32 icon_32x32@2x.png:64 \
  icon_128x128.png:128 icon_128x128@2x.png:256 \
  icon_256x256.png:256 icon_256x256@2x.png:512 \
  icon_512x512.png:512 icon_512x512@2x.png:1024; do
  icon_name="${spec%%:*}"
  icon_size="${spec##*:}"
  sips -z "$icon_size" "$icon_size" "$icon_square" --out "$iconset/$icon_name" >/dev/null
done
rm -f "$bundle/Contents/Resources/BetterDeck.icns"
iconutil --convert icns "$iconset" --output "$bundle/Contents/Resources/BetterDeck-Purple.icns"

cat > "$bundle/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key><string>pt_BR</string>
  <key>CFBundleExecutable</key><string>BetterDeck</string>
  <key>CFBundleIdentifier</key><string>dev.betterdeck.app</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleName</key><string>BetterDeck</string>
  <key>CFBundleIconFile</key><string>BetterDeck-Purple.icns</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>CFBundleVersion</key><string>2</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST
codesign --force --sign - "$bundle"
echo "$bundle"
