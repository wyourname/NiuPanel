#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <x86_64|aarch64|armv7> <output-dir>" >&2
    exit 2
fi

INPUT_ARCH=$1
OUTPUT_DIR=$2
UV_VERSION=${UV_VERSION:-0.8.15}
PNPM_VERSION=${PNPM_VERSION:-11.18.0}
GITHUB_RELEASE_MIRROR=${GITHUB_RELEASE_MIRROR:-https://git.365676.xyz/https://github.com}
PNPM_BOOTSTRAP_REGISTRY=${PNPM_BOOTSTRAP_REGISTRY:-https://registry.npmmirror.com}
PNPM_NODE_DIST_MIRROR=${PNPM_NODE_DIST_MIRROR:-https://mirrors.ustc.edu.cn/node}
NIUPANEL_RUNTIME_TOOLS=${NIUPANEL_RUNTIME_TOOLS:-all}
ARMV7_NODE_VERSION=22.23.1

case "$NIUPANEL_RUNTIME_TOOLS" in
    all|uv|pnpm) ;;
    *)
        echo "NIUPANEL_RUNTIME_TOOLS must be all, uv, or pnpm" >&2
        exit 2
        ;;
esac

case "$INPUT_ARCH" in
    amd64|x86_64)
        ARCH=x86_64
        UV_TRIPLE=x86_64-unknown-linux-gnu
        PNPM_PLATFORM_PACKAGE=@pnpm/linux-x64
        PNPM_PLATFORM_ARCHIVE=linux-x64
        PNPM_PLATFORM_SIZE=47159229
        PNPM_PLATFORM_SHA512=f70804928e17fe3c433a933ecdcbb909c26d6a6dbe9c4be4b84d5845100b744938870cc97636c2825f715ca215103d7d64262902e175cc0b7189772e931937c4
        ;;
    arm64|aarch64)
        ARCH=aarch64
        UV_TRIPLE=aarch64-unknown-linux-gnu
        PNPM_PLATFORM_PACKAGE=@pnpm/linux-arm64
        PNPM_PLATFORM_ARCHIVE=linux-arm64
        PNPM_PLATFORM_SIZE=47358242
        PNPM_PLATFORM_SHA512=716384af9c5b994459a63d97668ff7a6ab2bb434f3aa6b3c836ed9e1c881d7e71d8f7485f003edcf20bfcc9909ebe8139d1c12d2b2156a59ee952c030ca89ab0
        ;;
    arm|armv7|armhf)
        ARCH=armv7
        UV_TRIPLE=armv7-unknown-linux-gnueabihf
        PNPM_PLATFORM_PACKAGE=
        PNPM_PLATFORM_ARCHIVE=
        PNPM_PLATFORM_SIZE=
        PNPM_PLATFORM_SHA512=
        ;;
    *)
        echo "Unsupported runtime tool architecture: $INPUT_ARCH" >&2
        exit 2
        ;;
esac

for command_name in curl tar sha256sum sha512sum; do
    if ! command -v "$command_name" >/dev/null 2>&1; then
        echo "Missing required command: $command_name" >&2
        exit 1
    fi
done

mkdir -p "$OUTPUT_DIR"
OUTPUT_DIR=$(cd "$OUTPUT_DIR" && pwd)
STAGING_DIRS=()

cleanup() {
    local staging_dir
    for staging_dir in "${STAGING_DIRS[@]}"; do
        if [ -n "$staging_dir" ] && [ -d "$staging_dir" ]; then
            rm -rf -- "$staging_dir"
        fi
    done
}
trap cleanup EXIT

download_with_fallback() {
    local destination=$1
    shift
    local url

    rm -f -- "$destination"
    for url in "$@"; do
        [ -n "$url" ] || continue
        echo "   ↳ $url"
        if curl -fsSL --http1.1 --retry 5 --retry-delay 2 --retry-all-errors \
            --connect-timeout 20 --max-time 0 "$url" -o "$destination"; then
            return 0
        fi
        rm -f -- "$destination"
    done

    echo "All download sources failed: $*" >&2
    return 1
}

download_verified() {
    local algorithm=$1
    local expected_hash=$2
    local expected_size=$3
    local destination=$4
    shift 4
    local url actual_hash actual_size

    rm -f -- "$destination"
    for url in "$@"; do
        [ -n "$url" ] || continue
        echo "   ↳ $url"
        if ! curl -fsSL --http1.1 --retry 5 --retry-delay 2 --retry-all-errors \
            --connect-timeout 20 --max-time 0 "$url" -o "$destination"; then
            rm -f -- "$destination"
            continue
        fi

        actual_size=$(wc -c < "$destination" | tr -d '[:space:]')
        if [ "$actual_size" != "$expected_size" ]; then
            echo "Downloaded size mismatch: expected $expected_size, got $actual_size" >&2
            rm -f -- "$destination"
            continue
        fi

        case "$algorithm" in
            sha256) actual_hash=$(sha256sum "$destination" | awk '{print $1}') ;;
            sha512) actual_hash=$(sha512sum "$destination" | awk '{print $1}') ;;
            *)
                echo "Unsupported checksum algorithm: $algorithm" >&2
                return 1
                ;;
        esac
        if [ "$actual_hash" = "$expected_hash" ]; then
            return 0
        fi

        echo "Downloaded checksum mismatch for $url" >&2
        rm -f -- "$destination"
    done

    echo "All verified download sources failed" >&2
    return 1
}

prepare_uv() {
    local marker="$OUTPUT_DIR/.uv-version"
    local staging archive source

    if [ -x "$OUTPUT_DIR/uv" ] && [ "$(cat "$marker" 2>/dev/null || true)" = "$UV_VERSION" ]; then
        echo "✓ uv $UV_VERSION ($ARCH)"
        return
    fi

    echo "📦 Preparing uv $UV_VERSION ($ARCH)..."
    staging=$(mktemp -d "$OUTPUT_DIR/.uv-staging.XXXXXX")
    STAGING_DIRS+=("$staging")
    archive="$staging/uv.tar.gz"
    download_with_fallback "$archive" \
        "$GITHUB_RELEASE_MIRROR/astral-sh/uv/releases/download/$UV_VERSION/uv-$UV_TRIPLE.tar.gz" \
        "https://github.com/astral-sh/uv/releases/download/$UV_VERSION/uv-$UV_TRIPLE.tar.gz"
    tar -xzf "$archive" -C "$staging"
    source="$staging/uv-$UV_TRIPLE/uv"
    if [ ! -f "$source" ]; then
        echo "uv archive does not contain the expected executable" >&2
        return 1
    fi
    cp -f -- "$source" "$OUTPUT_DIR/.uv.new"
    chmod 0755 "$OUTPUT_DIR/.uv.new"
    mv -f -- "$OUTPUT_DIR/.uv.new" "$OUTPUT_DIR/uv"
    printf '%s\n' "$UV_VERSION" > "$marker"
}

prepare_pnpm() {
    local marker="$OUTPUT_DIR/.pnpm-version"
    local staging base_archive platform_archive package_root
    local registry npmjs_registry

    if [ -x "$OUTPUT_DIR/pnpm" ] &&
        [ -f "$OUTPUT_DIR/dist/pnpm.mjs" ] &&
        [ "$(cat "$marker" 2>/dev/null || true)" = "$PNPM_VERSION" ]; then
        if [ "$ARCH" != "armv7" ] || {
                [ -x "$OUTPUT_DIR/node-v$ARMV7_NODE_VERSION-linux-armv7l/bin/node" ]
        }; then
            echo "✓ pnpm $PNPM_VERSION ($ARCH)"
            return
        fi
    fi

    echo "📦 Preparing pnpm $PNPM_VERSION ($ARCH)..."
    staging=$(mktemp -d "$OUTPUT_DIR/.pnpm-staging.XXXXXX")
    STAGING_DIRS+=("$staging")
    base_archive="$staging/pnpm-base.tgz"
    registry=${PNPM_BOOTSTRAP_REGISTRY%/}
    npmjs_registry=https://registry.npmjs.org

    download_verified \
        sha512 \
        f5a3e07936a73db94f2c613457e4924c500498dd5a174cb7384fa23543bf456e685e865fc8f4b7e214830f00e6572f32a2403ff84bb041910d303c5a6880b2b0 \
        3988555 \
        "$base_archive" \
        "$registry/@pnpm/exe/-/exe-$PNPM_VERSION.tgz" \
        "$npmjs_registry/@pnpm/exe/-/exe-$PNPM_VERSION.tgz"
    tar -xzf "$base_archive" -C "$staging"
    package_root="$staging/package"

    if [ "$ARCH" = "armv7" ]; then
        local node_archive node_archive_name node_mirror
        node_archive_name="node-v$ARMV7_NODE_VERSION-linux-armv7l.tar.gz"
        node_archive="$staging/$node_archive_name"
        node_mirror=${PNPM_NODE_DIST_MIRROR%/}
        download_verified \
            sha256 \
            03c56ac0bd3ef3cce967c2f7b2f7ac2259a4ae7ceeaa661291aadf65729a8b53 \
            51527978 \
            "$node_archive" \
            "$node_mirror/v$ARMV7_NODE_VERSION/$node_archive_name" \
            "https://nodejs.org/dist/v$ARMV7_NODE_VERSION/$node_archive_name"
        tar -xzf "$node_archive" -C "$package_root"
        {
            printf '%s\n' '#!/bin/sh'
            printf '%s\n' 'base=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)'
            printf '%s\n' "exec \"\$base/node-v$ARMV7_NODE_VERSION-linux-armv7l/bin/node\" \"\$base/dist/pnpm.mjs\" \"\$@\""
        } > "$package_root/pnpm"
        chmod 0755 "$package_root/pnpm"

        rm -rf -- "$OUTPUT_DIR/node-v$ARMV7_NODE_VERSION-linux-armv7l"
        cp -a -- \
            "$package_root/node-v$ARMV7_NODE_VERSION-linux-armv7l" \
            "$OUTPUT_DIR/node-v$ARMV7_NODE_VERSION-linux-armv7l"
    else
        platform_archive="$staging/pnpm-platform.tgz"
        download_verified \
            sha512 \
            "$PNPM_PLATFORM_SHA512" \
            "$PNPM_PLATFORM_SIZE" \
            "$platform_archive" \
            "$registry/$PNPM_PLATFORM_PACKAGE/-/$PNPM_PLATFORM_ARCHIVE-$PNPM_VERSION.tgz" \
            "$npmjs_registry/$PNPM_PLATFORM_PACKAGE/-/$PNPM_PLATFORM_ARCHIVE-$PNPM_VERSION.tgz"
        tar -xzf "$platform_archive" -C "$staging"
    fi

    if [ ! -f "$package_root/pnpm" ]; then
        echo "pnpm archive does not contain the expected executable" >&2
        return 1
    fi
    if [ ! -f "$package_root/dist/pnpm.mjs" ]; then
        echo "pnpm archive does not contain dist/pnpm.mjs" >&2
        return 1
    fi
    rm -rf -- "$OUTPUT_DIR/dist"
    cp -a -- "$package_root/dist" "$OUTPUT_DIR/dist"
    cp -f -- "$package_root/pnpm" "$OUTPUT_DIR/.pnpm.new"
    chmod 0755 "$OUTPUT_DIR/.pnpm.new"
    mv -f -- "$OUTPUT_DIR/.pnpm.new" "$OUTPUT_DIR/pnpm"
    printf '%s\n' "$PNPM_VERSION" > "$marker"
}

# Old caches must never leak fnm back into a new update package.
rm -f -- "$OUTPUT_DIR/fnm"
case "$NIUPANEL_RUNTIME_TOOLS" in
    all)
        prepare_uv
        prepare_pnpm
        ;;
    uv)
        prepare_uv
        ;;
    pnpm)
        prepare_pnpm
        ;;
esac
