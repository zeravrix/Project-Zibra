#!/usr/bin/env bash
set -e
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ -d "$HOME/.local/usr/lib64" ]; then
    export PKG_CONFIG_PATH="$HOME/.local/usr/lib64/pkgconfig:${PKG_CONFIG_PATH:-}"
    export LIBRARY_PATH="$HOME/.local/usr/lib64:${LIBRARY_PATH:-}"
    export LD_LIBRARY_PATH="$HOME/.local/usr/lib64:${LD_LIBRARY_PATH:-}"
fi

if [ -x "$DIR/bin/zibra-desktop" ]; then
    exec "$DIR/bin/zibra-desktop" "$@"
elif [ -x "$DIR/zibra-desktop" ]; then
    exec "$DIR/zibra-desktop" "$@"
elif [ -x "$DIR/target/release/zibra-desktop" ]; then
    exec "$DIR/target/release/zibra-desktop" "$@"
else
    echo "ZibraX binary not found. Expected in bin/ or target/release/." >&2
    exit 1
fi
