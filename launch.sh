#!/usr/bin/env bash
set -e
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export PKG_CONFIG_PATH=/home/u1m/.local/usr/lib64/pkgconfig:${PKG_CONFIG_PATH:-}
export LIBRARY_PATH=/home/u1m/.local/usr/lib64:${LIBRARY_PATH:-}
export LD_LIBRARY_PATH=/home/u1m/.local/usr/lib64:${LD_LIBRARY_PATH:-}
exec "$DIR/target/release/zibra-desktop" "$@"
