#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
RELEASE_APP="$ROOT/target/release/bundle/macos/Amara Unfold.app"
DEBUG_APP="$ROOT/target/debug/bundle/macos/Amara Unfold.app"
RELEASE_CLI="$ROOT/target/release/amara-unfold"
DEBUG_CLI="$ROOT/target/debug/amara-unfold"
VERSION=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)
DMG_SOURCE=""
DMG_FILE=""

if [ -d "$RELEASE_APP" ] && [ -f "$RELEASE_CLI" ]; then
  APP="$RELEASE_APP"
  CLI="$RELEASE_CLI"
  BUILD_PROFILE=release
else
  APP="$DEBUG_APP"
  CLI="$DEBUG_CLI"
  BUILD_PROFILE=debug
fi

BUNDLE_BUILD=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleVersion' "$APP/Contents/Info.plist")
DMG_CANDIDATE="$ROOT/target/$BUILD_PROFILE/bundle/dmg/Amara Unfold_${VERSION}_aarch64.dmg"
if [ -f "$DMG_CANDIDATE" ]; then
  DMG_SOURCE="$DMG_CANDIDATE"
  DMG_FILE=$(basename "$DMG_SOURCE")
fi
if [ -n "$DMG_FILE" ]; then
  INSTALLER_DMG_JSON="\"$DMG_FILE\""
else
  INSTALLER_DMG_JSON=null
fi
STAMP=$(date '+%Y%m%d-%H%M%S')
NAME="Amara-Unfold-${VERSION}-macOS-arm64-${STAMP}"
DIST="$ROOT/dist"
OUT="$DIST/$NAME"

if [ ! -d "$APP" ]; then
  printf '%s\n' "Missing app bundle: $APP" >&2
  printf '%s\n' "Run: npm run tauri --workspace @amara-unfold/desktop -- build" >&2
  exit 1
fi

if [ ! -f "$CLI" ]; then
  printf '%s\n' "Missing CLI binary: $CLI" >&2
  printf '%s\n' "Run: cargo build -p amara-unfold-adapters --bin amara-unfold --release" >&2
  exit 1
fi

case $(file "$APP/Contents/MacOS/amara-unfold-desktop") in
  *arm64*) ;;
  *) printf '%s\n' "Desktop binary is not arm64" >&2; exit 1 ;;
esac

case $(file "$CLI") in
  *arm64*) ;;
  *) printf '%s\n' "CLI binary is not arm64" >&2; exit 1 ;;
esac

mkdir -p "$DIST" "$OUT"

COPYFILE_DISABLE=1 ditto -c -k --keepParent "$APP" "$OUT/Amara Unfold.app.zip"
cp "$CLI" "$OUT/amara-unfold"
chmod 755 "$OUT/amara-unfold"
if [ -n "$DMG_SOURCE" ]; then
  cp "$DMG_SOURCE" "$OUT/$DMG_FILE"
fi
cp "$ROOT/docs/transfer-readme-first.md" "$OUT/README-FIRST.md"
cp "$ROOT/docs/second-mac-llm-instructions.md" "$OUT/LLM-INSTRUCTIONS.md"
cp "$ROOT/docs/component-manifest.md" "$OUT/COMPONENTS.md"
cp "$ROOT/docs/how-to-use.md" "$OUT/HOW-TO-USE.md"
cp "$ROOT/docs/claude-local-app-approval.md" "$OUT/CLAUDE-LOCAL-APP-APPROVAL.md"
cp "$ROOT/docs/local-app-uat.md" "$OUT/LOCAL-APP-UAT.md"
cp "$ROOT/docs/privacy.md" "$OUT/PRIVACY.md"
cp "$ROOT/docs/open-adapters.md" "$OUT/OPEN-ADAPTERS.md"
cp "$ROOT/LICENSE" "$OUT/LICENSE"
node "$ROOT/scripts/local-app-uat-status.mjs" --portable >"$OUT/LOCAL-APP-UAT-STATUS.txt"
node "$ROOT/scripts/local-app-uat-status.mjs" --portable --json >"$OUT/LOCAL-APP-UAT-STATUS.json"

GIT_REVISION=$(git -C "$ROOT" rev-parse HEAD 2>/dev/null || printf 'unavailable')
if [ -z "$(git -C "$ROOT" status --porcelain 2>/dev/null)" ]; then
  SOURCE_STATE=clean
else
  SOURCE_STATE=modified
fi

cat >"$OUT/BUILD-INFO.txt" <<EOF
Product: Amara Unfold
Version: $VERSION
Bundle build: $BUNDLE_BUILD
Package: $NAME
Created: $(date -u '+%Y-%m-%dT%H:%M:%SZ')
Target: macOS arm64
Minimum documented macOS: 11
Build profile: $BUILD_PROFILE
Signing: unsigned/ad-hoc, not notarized
Installer DMG: ${DMG_FILE:-not included}
Bundle identifier: com.amaramemory.unfold
Source revision: $GIT_REVISION
Source state at packaging: $SOURCE_STATE
Runtime network: disabled
Telemetry: disabled
Personal data included: no
EOF

cat >"$OUT/manifest.json" <<EOF
{
  "schemaVersion": 1,
  "product": "Amara Unfold",
  "version": "$VERSION",
  "bundleBuild": "$BUNDLE_BUILD",
  "target": "macOS-arm64",
  "buildProfile": "$BUILD_PROFILE",
  "signed": false,
  "notarized": false,
  "runtimeNetwork": false,
  "telemetry": false,
  "personalDataIncluded": false,
  "desktopArchive": "Amara Unfold.app.zip",
  "installerDmg": $INSTALLER_DMG_JSON,
  "cli": "amara-unfold",
  "llmInstructions": "LLM-INSTRUCTIONS.md",
  "approvalChecklist": "CLAUDE-LOCAL-APP-APPROVAL.md",
  "localAppUat": "LOCAL-APP-UAT.md",
  "localAppUatStatusText": "LOCAL-APP-UAT-STATUS.txt",
  "localAppUatStatusJson": "LOCAL-APP-UAT-STATUS.json",
  "firstRead": "README-FIRST.md"
}
EOF

(
  cd "$OUT"
  find . -type f ! -name CHECKSUMS-SHA256.txt -print \
    | LC_ALL=C sort \
    | sed 's#^./##' \
    | while IFS= read -r file_name; do
        shasum -a 256 "$file_name"
      done >CHECKSUMS-SHA256.txt
)

find "$OUT" -type f -name '._*' -delete
rm -f "$DIST/$NAME.zip"
(
  cd "$DIST"
  COPYFILE_DISABLE=1 zip -r -X "$NAME.zip" "$NAME" >/dev/null
)

printf '%s\n' "$OUT"
printf '%s\n' "$DIST/$NAME.zip"
