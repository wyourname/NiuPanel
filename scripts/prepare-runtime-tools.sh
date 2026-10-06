#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <x86_64|aarch64|armv7> <output-dir>" >&2
    exit 2
fi

INPUT_ARCH=$1
OUTPUT_DIR=$2
UV_VERSION=${UV_VERSION:-0.8.15}
PNPM_VERSION=${PNPM_VERSION:-12.9.1}
GITHUB_RELEASE_MIRROR=${GITHUB_RELEASE_MIRROR:-https://git.365676.xyz/https://github.com}
PNPM_BOOTSTRAP_REGISTRY=${PNPM_BOOTSTRAP_REGISTRY:-https://registry.npmmirror.com}
PNPM_NODE_DIST_MIRROR=${PNPM_NODE_DIST_MIRROR:-https://mirrors.ustc.edu.cn/node}
NIUPANEL_RUNTIME_TOOLS=${NIUPANEL_RUNTIME_TOOLS:-all}

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
        PNPM_PLATFORM_PACKAGE=@pnpm/exe.linux-x64
        PNPM_PLATFORM_ARCHIVE=exe.linux-x64
        PNPM_PLATFORM_SIZE=23369948
        PNPM_PLATFORM_SHA512=f8ce4def884ce1f73cec001c931c7f5085be7549d67f67a4a3e1dc849b3f3595317a12e17a4372308fc635a35796f77e91647ef7b5d7043e755d192436252dac
        ;;
    arm64|aarch64)
        ARCH=aarch64
        UV_TRIPLE=aarch64-unknown-linux-gnu
        PNPM_PLATFORM_PACKAGE=@pnpm/exe.linux-arm64
        PNPM_PLATFORM_ARCHIVE=exe.linux-arm64
        PNPM_PLATFORM_SIZE=21515849
        PNPM_PLATFORM_SHA512=731fae17ac6ac04b66ec1f0d57ed6f7b4c4183d81cbb3419e51fb45a0396bca1bf87defc5258d2e81816b358a9206b8681475e5619b73ca98196d5c871cccb5c
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

    if [ "$ARCH" = "armv7" ]; then
        echo "pnpm 12 upstream does not publish a Linux ARMv7 binary; use amd64/arm64 for the bundled Docker runtime." >&2
        return 1
    fi
    if [ -x "$OUTPUT_DIR/pnpm" ] &&
        [ -d "$OUTPUT_DIR/dist" ] &&
        [ "$(cat "$marker" 2>/dev/null || true)" = "$PNPM_VERSION" ]; then
        echo "✓ pnpm $PNPM_VERSION ($ARCH)"
        return
    fi

    echo "📦 Preparing pnpm $PNPM_VERSION ($ARCH)..."
    staging=$(mktemp -d "$OUTPUT_DIR/.pnpm-staging.XXXXXX")
    STAGING_DIRS+=("$staging")
    base_archive="$staging/pnpm-base.tgz"
    registry=${PNPM_BOOTSTRAP_REGISTRY%/}
    npmjs_registry=https://registry.npmjs.org

    download_verified \
        sha512 \
        4d9395bd8abc9f0ce6c3c211dcfd4629993ceed57bbc33c24f7a0994bec886060d65f2c6a2fe235f427c449c6e017c484f7bb183fba27289eb2cb06ff97e4ac8 \
        908979 \
        "$base_archive" \
        "$registry/@pnpm/exe/-/exe-$PNPM_VERSION.tgz" \
        "$npmjs_registry/@pnpm/exe/-/exe-$PNPM_VERSION.tgz"
    tar -xzf "$base_archive" -C "$staging"
    package_root="$staging/package"

    platform_archive="$staging/pnpm-platform.tgz"
    download_verified \
        sha512 \
        "$PNPM_PLATFORM_SHA512" \
        "$PNPM_PLATFORM_SIZE" \
        "$platform_archive" \
        "$registry/$PNPM_PLATFORM_PACKAGE/-/$PNPM_PLATFORM_ARCHIVE-$PNPM_VERSION.tgz" \
        "$npmjs_registry/$PNPM_PLATFORM_PACKAGE/-/$PNPM_PLATFORM_ARCHIVE-$PNPM_VERSION.tgz"
    tar -xzf "$platform_archive" -C "$staging"

    if [ ! -f "$package_root/pnpm" ]; then
        echo "pnpm archive does not contain the expected executable" >&2
        return 1
    fi
    if [ ! -d "$package_root/dist" ]; then
        echo "pnpm archive does not contain runtime support files" >&2
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
