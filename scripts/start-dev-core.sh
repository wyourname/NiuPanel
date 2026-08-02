#!/usr/bin/env bash
set -euo pipefail

# Keep the development container on the same process and release path as the
# production image: Launcher owns the runtime database, activation, health
# checks, and rollback boundary. The Vite service remains responsible for the
# live frontend on port 7787.
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

core_version="$(awk -F'"' '/^[[:space:]]*version[[:space:]]*=/ { print $2; exit }' niupanel/Cargo.toml)"
web_version="$(node -p "require('./niupanelweb/package.json').version")"
if [[ -z "$core_version" || -z "$web_version" ]]; then
    echo "Unable to determine development Panel versions" >&2
    exit 1
fi

export NIUPANEL_BUNDLED_CORE="${NIUPANEL_BUNDLED_CORE:-$ROOT_DIR/target/debug/niupanel}"
export NIUPANEL_SYSTEM_DIR="${NIUPANEL_SYSTEM_DIR:-$ROOT_DIR/data/system}"

cargo build -p niupanel -p niupanel-launcher

# Reuse the active development release when the compiled Core and bootstrap
# Web entrypoint are unchanged. This keeps runtime.db and its rollback chain
# useful without copying the large debug binary on every restart.
active_version=""
active_web_version=""
runtime_db="$NIUPANEL_SYSTEM_DIR/runtime.db"
if [[ -f "$runtime_db" ]]; then
    active_release="$(sqlite3 -separator $'\t' "$runtime_db" \
        "SELECT r.version, r.web_version
           FROM panel_runtime_state AS s
           JOIN panel_releases AS r ON r.version = s.active_version
          WHERE s.id = 1;" 2>/dev/null || true)"
    IFS=$'\t' read -r active_version active_web_version <<<"$active_release"
fi

reuse_active=false
if [[ "$active_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+-dev\.[0-9]+$ ]]; then
    active_core="$NIUPANEL_SYSTEM_DIR/releases/panel/$active_version/core/niupanel"
    active_web_index="$NIUPANEL_SYSTEM_DIR/releases/panel/$active_version/web/index.html"
    if cmp -s "$NIUPANEL_BUNDLED_CORE" "$active_core" \
        && cmp -s "$ROOT_DIR/niupanelweb/index.html" "$active_web_index"; then
        reuse_active=true
    fi
fi

if [[ "$reuse_active" == true ]]; then
    export NIUPANEL_BOOTSTRAP_PANEL_VERSION="${NIUPANEL_BOOTSTRAP_PANEL_VERSION:-$active_version}"
    export NIUPANEL_BOOTSTRAP_CORE_VERSION="${NIUPANEL_BOOTSTRAP_CORE_VERSION:-$active_version}"
    export NIUPANEL_BOOTSTRAP_WEB_VERSION="${NIUPANEL_BOOTSTRAP_WEB_VERSION:-$active_web_version}"
else
    # A timestamped prerelease is used only when the development bundle
    # changes, so Launcher can activate it through the normal immutable queue.
    dev_suffix="${NIUPANEL_DEV_RELEASE_SUFFIX:-$(date +%s%N)}"
    export NIUPANEL_BOOTSTRAP_PANEL_VERSION="${NIUPANEL_BOOTSTRAP_PANEL_VERSION:-${core_version}-dev.${dev_suffix}}"
    export NIUPANEL_BOOTSTRAP_CORE_VERSION="${NIUPANEL_BOOTSTRAP_CORE_VERSION:-${core_version}-dev.${dev_suffix}}"
    export NIUPANEL_BOOTSTRAP_WEB_VERSION="${NIUPANEL_BOOTSTRAP_WEB_VERSION:-${web_version}-dev.${dev_suffix}}"
fi

# Launcher requires a static Web root for the immutable bootstrap release.
# Development UI itself is served by Vite, so only the source entrypoint is
# needed here; this deliberately avoids copying node_modules into runtime data.
dev_web_root="$(mktemp -d "${TMPDIR:-/tmp}/niupanel-dev-web.XXXXXX")"
cp "$ROOT_DIR/niupanelweb/index.html" "$dev_web_root/index.html"
export BUNDLED_WEB_DIR="${BUNDLED_WEB_DIR:-$dev_web_root}"

exec "$ROOT_DIR/target/debug/niupanel-launcher"
