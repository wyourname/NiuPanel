#!/bin/bash
set -e

# === 配置区域 ===
APP_NAME="niupanel"
FRONTEND_DIR="niupanelweb"
TOOLS_CACHE_DIR="release_tools"
CORE_VERSION=$(sed -n '/^\[package\]/,/^\[/{s/^version = "\([^"]*\)"/\1/p}' niupanel/Cargo.toml | head -n 1)

node scripts/verify-version-contract.mjs

prepare_runtime_tools() {
    local arch_name=$1
    local target_dir="$TOOLS_CACHE_DIR/$arch_name"
    scripts/prepare-runtime-tools.sh "$arch_name" "$target_dir"
}

# 帮助信息
usage() {
    echo "用法: $0 [amd64|arm64|armv7|all]"
    echo "  amd64   : 构建 x86_64 架构"
    echo "  arm64   : 构建 aarch64 架构"
    echo "  armv7   : 构建 armv7 架构"
    echo "  all     : 构建所有支持的架构"
    echo "  (空)    : 构建本机架构"
}

# 1. 构建前端 (只构建一次)
build_frontend() {
    echo "⚡ 正在构建前端..."
    if [ -d "$FRONTEND_DIR" ]; then
        if ! command -v pnpm > /dev/null 2>&1; then
            echo "❌ 前端构建需要 pnpm 11.18+，请先安装 pnpm"
            exit 1
        fi
        cd "$FRONTEND_DIR"
        echo "📦 正在同步前端依赖..."
        pnpm install --frozen-lockfile
        NIUPANEL_WEB_CORE_MIN="$CORE_VERSION" pnpm run build
        cd ..

        if [ ! -d "$FRONTEND_DIR/dist" ]; then
             echo "❌ 错误: 前端构建产物 (dist) 未生成"
             exit 1
        fi
    else
        echo "⚠️ 警告: 未找到前端目录 $FRONTEND_DIR，将不包含网页文件"
    fi
}

# 2. 构建指定架构的后端并打包
build_backend_and_package() {
    local TARGET_ALIAS=$1
    local OUTPUT_DIR="release_dist"
    local BUILD_CMD=""
    local ARCH_NAME=""
    local BIN_SOURCE_PATH=""
    local TARGET_TRIPLE=""

    export RUSTFLAGS="-C target-feature=+crt-static"

    # === 优化: 修复 Docker 内部无法访问宿主机 127.0.0.1 代理的问题 ===
    if [[ "$HTTP_PROXY" == *"127.0.0.1"* || "$http_proxy" == *"127.0.0.1"* ]]; then
        echo "🌐 检测到无效的本地代理配置，正在尝试清除以确保下载成功..."
        unset http_proxy https_proxy ALL_PROXY HTTP_PROXY HTTPS_PROXY all_proxy
    fi

    # 确定目标 Triple 和 架构名称
    if [ "$TARGET_ALIAS" == "host" ]; then
        echo "🏗️  正在构建本机架构..."
        export CARGO_TARGET_DIR="target"
        BUILD_CMD="cargo build --release -p niupanel -p niupanel-launcher"
        ARCH_NAME=$(uname -m)
        TARGET_TRIPLE=$(rustc -vV | sed -n 's/^host: //p')
        BIN_SOURCE_PATH="target/release"

    elif [ "$TARGET_ALIAS" == "amd64" ]; then
        echo "🏗️  正在构建 AMD64 (x86_64)..."
        export CARGO_TARGET_DIR="target_amd64"
        TARGET_TRIPLE="x86_64-unknown-linux-musl"
        ARCH_NAME="x86_64"
        BIN_SOURCE_PATH="$CARGO_TARGET_DIR/$TARGET_TRIPLE/release"

        if ! command -v cross &> /dev/null; then echo "❌ 需要安装 cross 工具 (cargo install cross)"; exit 1; fi
        BUILD_CMD="cross build --release --target $TARGET_TRIPLE -p niupanel -p niupanel-launcher"

    elif [ "$TARGET_ALIAS" == "arm64" ]; then
        echo "🏗️  正在构建 ARM64 (aarch64)..."
        export CARGO_TARGET_DIR="target_arm64"
        TARGET_TRIPLE="aarch64-unknown-linux-musl"
        ARCH_NAME="aarch64"
        BIN_SOURCE_PATH="$CARGO_TARGET_DIR/$TARGET_TRIPLE/release"

        if ! command -v cross &> /dev/null; then echo "❌ 需要安装 cross 工具 (cargo install cross)"; exit 1; fi
        BUILD_CMD="cross build --release --target $TARGET_TRIPLE -p niupanel -p niupanel-launcher"

    elif [ "$TARGET_ALIAS" == "armv7" ]; then
        echo "🏗️  正在构建 ARMv7..."
        export CARGO_TARGET_DIR="target_armv7"
        TARGET_TRIPLE="armv7-unknown-linux-musleabihf"
        ARCH_NAME="armv7"
        BIN_SOURCE_PATH="$CARGO_TARGET_DIR/$TARGET_TRIPLE/release"

        if ! command -v cross &> /dev/null; then echo "❌ 需要安装 cross 工具 (cargo install cross)"; exit 1; fi
        BUILD_CMD="cross build --release --target $TARGET_TRIPLE -p niupanel -p niupanel-launcher"
    else
        echo "❌ 未知架构: $TARGET_ALIAS"
        exit 1
    fi

    echo "▶️  执行命令: $BUILD_CMD"
    eval $BUILD_CMD

    # 打包流程
    mkdir -p docker
    TAR_NAME="docker/${APP_NAME}_linux_${ARCH_NAME}.tar.gz"
    # 使用带架构后缀的临时目录防止冲突
    TEMP_DIR="${OUTPUT_DIR}_${ARCH_NAME}"

    rm -rf "$TEMP_DIR"
    rm -f "$TAR_NAME"
    mkdir -p "$TEMP_DIR/web" "$TEMP_DIR/tools"

    # 复制二进制
    if [ -f "$BIN_SOURCE_PATH/$APP_NAME" ]; then
        cp "$BIN_SOURCE_PATH/$APP_NAME" "$TEMP_DIR/"
        echo "✅ 二进制文件已复制"
    else
        echo "❌ 错误: 找不到编译产物 $BIN_SOURCE_PATH/$APP_NAME"
        exit 1
    fi
    if [ -f "$BIN_SOURCE_PATH/niupanel-launcher" ]; then
        cp "$BIN_SOURCE_PATH/niupanel-launcher" "$TEMP_DIR/"
        echo "✅ Launcher 已复制"
    else
        echo "❌ 错误: 找不到编译产物 $BIN_SOURCE_PATH/niupanel-launcher"
        exit 1
    fi
    node scripts/generate-core-release-manifest.mjs \
        "$TEMP_DIR/$APP_NAME" \
        "$CORE_VERSION" \
        "$TARGET_TRIPLE" \
        "$TEMP_DIR/core-release.json"

    # 复制前端 (假设已构建)
    if [ -d "$FRONTEND_DIR/dist" ]; then
        cp -r "$FRONTEND_DIR/dist/." "$TEMP_DIR/web/"
    fi

    prepare_runtime_tools "$ARCH_NAME"
    cp -a "$TOOLS_CACHE_DIR/$ARCH_NAME/." "$TEMP_DIR/tools/"
    rm -f "$TEMP_DIR/tools/.uv-version" "$TEMP_DIR/tools/.pnpm-version" "$TEMP_DIR/tools/fnm"
    test -x "$TEMP_DIR/tools/uv"
    test -x "$TEMP_DIR/tools/pnpm"

    echo "📦 打包为 $TAR_NAME ..."
    tar -czvf "$TAR_NAME" -C "$TEMP_DIR" .
    rm -rf "$TEMP_DIR"
    echo "✨ 构建完成: $TAR_NAME"
    echo "----------------------------------------"
}

# === 主流程 ===
INPUT_ARG="${1:-}"

# 始终先检查并构建前端
build_frontend

if [ -z "$INPUT_ARG" ]; then
    build_backend_and_package "host"
else
    case "$INPUT_ARG" in
        amd64)
            build_backend_and_package "amd64"
            ;;
        arm64)
            build_backend_and_package "arm64"
            ;;
        armv7)
            build_backend_and_package "armv7"
            ;;
        all)
            build_backend_and_package "amd64"
            build_backend_and_package "arm64"
            build_backend_and_package "armv7"
            ;;
        *)
            usage
            exit 1
            ;;
    esac
fi

echo "🎉 所有任务执行完毕！"
