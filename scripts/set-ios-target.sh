#!/bin/bash
# ──────────────────────────────────────────────────────────────────────
# set-ios-target.sh — Patch SDKROOT in the Xcode project before building
#
# WHY THIS EXISTS (Intel Mac x86_64):
#   • Real device builds need  SDKROOT = iphoneos
#   • Simulator builds need    SDKROOT = iphonesimulator
#   Xcode's -sdk flag on the command line should handle this, but the
#   archive step (used by cargo-mobile2 / tauri) reads the project-level
#   SDKROOT and ignores the CLI flag, causing linker failures:
#     - iphoneos SDK has no x86_64 Swift runtime → hundreds of missing
#       _swift_* symbols when archiving for simulator
#     - iphonesimulator SDK has no arm64 device libs → exit code 70
#       when building for a real device
#
# USAGE:
#   ./scripts/set-ios-target.sh simulator   # before pnpm ios:dev / ios:dev:ipad
#   ./scripts/set-ios-target.sh device      # before pnpm ios:device
# ──────────────────────────────────────────────────────────────────────

set -euo pipefail

PBXPROJ="src-tauri/gen/apple/test_app.xcodeproj/project.pbxproj"

if [[ ! -f "$PBXPROJ" ]]; then
  echo "⚠️  $PBXPROJ not found — skipping SDKROOT patch"
  exit 0
fi

TARGET="${1:-}"

case "$TARGET" in
  simulator)
    echo "🔧 Setting SDKROOT = iphonesimulator in project.pbxproj"
    sed -i '' 's/SDKROOT = iphoneos;/SDKROOT = iphonesimulator;/g' "$PBXPROJ"
    ;;
  device)
    echo "🔧 Setting SDKROOT = iphoneos in project.pbxproj"
    sed -i '' 's/SDKROOT = iphonesimulator;/SDKROOT = iphoneos;/g' "$PBXPROJ"
    ;;
  *)
    echo "Usage: $0 <simulator|device>"
    echo ""
    echo "  simulator  — sets SDKROOT = iphonesimulator (for iPad/iPhone simulator on Intel Mac)"
    echo "  device     — sets SDKROOT = iphoneos (for real device builds)"
    exit 1
    ;;
esac

echo "✅ Done"
