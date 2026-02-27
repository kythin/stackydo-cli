#!/usr/bin/env bash
set -euo pipefail

# Build local archives using cargo-dist and optionally create a GitHub release.
#
# Usage: ./build-local.sh [--all] [--no-release]
#   --all         Build all targets (requires cross-compilation setup)
#   --no-release  Build only, don't create GitHub release
#   (default)     Build macOS targets only + create release

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# Parse flags
BUILD_ALL=false
CREATE_RELEASE=true
for arg in "$@"; do
  case "$arg" in
    --all)        BUILD_ALL=true ;;
    --no-release) CREATE_RELEASE=false ;;
    *)            echo "Unknown flag: $arg"; exit 1 ;;
  esac
done

# Read config
CONFIG="$SCRIPT_DIR/.homebrew-tap.json"
if [ ! -f "$CONFIG" ]; then
  echo "ERROR: .homebrew-tap.json not found"
  exit 1
fi

NAME=$(jq -r '.name' "$CONFIG")
REPO=$(jq -r '.repo' "$CONFIG")

# Read version from Cargo.toml
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
TAG="v${VERSION}"

echo "=== Building $NAME $TAG ==="

# Determine targets
ARCH=$(uname -m)
if [ "$BUILD_ALL" = true ]; then
  # Build all targets from config
  TARGETS=$(jq -r '.targets[]' "$CONFIG")
else
  # Build only macOS targets
  TARGETS=$(jq -r '.targets[] | select(contains("apple-darwin"))' "$CONFIG")
fi

# Build each target
for target in $TARGETS; do
  echo ""
  echo "--- Building $target ---"

  # Check if this is a native target we can build directly
  case "$target" in
    aarch64-apple-darwin)
      if [ "$ARCH" != "arm64" ]; then
        echo "SKIP: $target requires arm64 Mac (current: $ARCH)"
        continue
      fi
      ;;
    x86_64-apple-darwin)
      # Rosetta or native x86_64 can build this on macOS
      if [[ "$(uname -s)" != "Darwin" ]]; then
        echo "SKIP: $target requires macOS"
        continue
      fi
      ;;
    *linux*|*windows*)
      if [ "$BUILD_ALL" != true ]; then
        echo "SKIP: $target (use --all to include non-macOS targets)"
        continue
      fi
      ;;
  esac

  dist build --artifacts=local --target "$target" 2>&1
  echo "OK: $target"
done

echo ""

# Collect built archives
DIST_DIR="$SCRIPT_DIR/target/distrib"
if [ ! -d "$DIST_DIR" ]; then
  echo "ERROR: No dist output found at $DIST_DIR"
  exit 1
fi

echo "Built archives:"
ls -la "$DIST_DIR"/*.tar.xz "$DIST_DIR"/*.zip 2>/dev/null || echo "(none found)"

if [ "$CREATE_RELEASE" = false ]; then
  echo ""
  echo "=== Build complete (--no-release, skipping GitHub release) ==="
  exit 0
fi

echo ""
echo "--- Creating GitHub release $TAG ---"

# Check if release already exists
if gh release view "$TAG" --repo "$REPO" &>/dev/null; then
  echo "Release $TAG already exists, uploading/replacing assets..."
  gh release upload "$TAG" --repo "$REPO" --clobber "$DIST_DIR"/*.tar.xz "$DIST_DIR"/*.zip 2>/dev/null || \
  gh release upload "$TAG" --repo "$REPO" --clobber "$DIST_DIR"/*.tar.xz 2>/dev/null || true
else
  # Create release with archives
  gh release create "$TAG" \
    --repo "$REPO" \
    --title "$TAG" \
    --generate-notes \
    "$DIST_DIR"/*.tar.xz "$DIST_DIR"/*.zip 2>/dev/null || \
  gh release create "$TAG" \
    --repo "$REPO" \
    --title "$TAG" \
    --generate-notes \
    "$DIST_DIR"/*.tar.xz 2>/dev/null || true
fi

echo ""
echo "=== $NAME $TAG released ==="
