#!/usr/bin/env bash
set -Eeuo pipefail

# Backward-compatible entry point. Prefer scripts/verify-web-dist.sh for new usage.
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

if (($# == 0)); then
    exec "$SCRIPT_DIR/scripts/verify-web-dist.sh" --serve
fi

if [[ ${1:-} =~ ^[0-9]+$ ]]; then
    exec "$SCRIPT_DIR/scripts/verify-web-dist.sh" --port "$1" --serve
fi

exec "$SCRIPT_DIR/scripts/verify-web-dist.sh" "$@"
