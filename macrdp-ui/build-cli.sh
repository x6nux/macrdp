#!/bin/bash
# Build macrdp-server CLI and copy to Tauri binaries directory
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TARGET_TRIPLE=$(rustc -vV | grep host | awk '{print $2}')

# 优先用环境变量 MACRDP_CLI_PROFILE，其次用参数，默认 debug
PROFILE="${MACRDP_CLI_PROFILE:-${1:-debug}}"

echo "Building macrdp-server (profile: $PROFILE, target: $TARGET_TRIPLE)..."

if [ "$PROFILE" = "release" ]; then
    cargo build -p macrdp-server --release --manifest-path "$PROJECT_ROOT/Cargo.toml"
    SRC="$PROJECT_ROOT/target/release/macrdp-server"
else
    cargo build -p macrdp-server --manifest-path "$PROJECT_ROOT/Cargo.toml"
    SRC="$PROJECT_ROOT/target/debug/macrdp-server"
fi

DEST="$SCRIPT_DIR/src-tauri/binaries/macrdp-server-$TARGET_TRIPLE"
mkdir -p "$(dirname "$DEST")"
cp "$SRC" "$DEST"
echo "Copied $SRC -> $DEST"
