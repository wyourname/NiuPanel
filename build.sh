#!/bin/bash
set -e

# === 配置区域 ===
APP_NAME="niupanel"
FRONTEND_DIR="niupanelweb"
TOOLS_CACHE_DIR="release_tools"
UV_VERSION="${UV_VERSION:-0.8.15}"
FNM_VERSION="${FNM_VERSION:-1.38.1}"
FNM_RELEASE_TAG="v${FNM_VERSION#v}"
GITHUB_RELEASE_MIRROR="${GITHUB_RELEASE_MIRROR:-https://git.365676.xyz/https://github.com}"

tool_versions_for_arch() {
    local arch_name=$1
    case "$arch_name" in
        x86_64)
            echo "x86_64-unknown-linux-gnu fnm-linux.zip"
            ;;
        aarch64)
            echo "aarch64-unknown-linux-gnu fnm-arm64.zip"
            ;;
        armv7)
            echo "armv7-unknown-linux-gnueabihf fnm-arm32.zip"
            ;;
        *)
            echo "❌ 不支持的工具架构: $arch_name" >&2
            return 1
            ;;
    esac
}

prepare_runtime_tools() {
    local arch_name=$1
    local target_dir="$TOOLS_CACHE_DIR/$arch_name"
    local uv_triple
    local fnm_pkg

    read -r uv_triple fnm_pkg < <(tool_versions_for_arch "$arch_name")
    mkdir -p "$target_dir"

    if [ ! -f "$target_dir/uv" ]; then
        echo "📦 下载 uv ($arch_name)..."
        download_with_fallback "$target_dir/uv.tar.gz" \
            "https://github.com/astral-sh/uv/releases/download/${UV_VERSION}/uv-${uv_triple}.tar.gz" \
            "${GITHUB_RELEASE_MIRROR}/astral-sh/uv/releases/download/${UV_VERSION}/uv-${uv_triple}.tar.gz"
        mkdir -p "$target_dir/uv_extract"
        tar -xzf "$target_dir/uv.tar.gz" -C "$target_dir/uv_extract"
        cp "$target_dir/uv_extract/uv-${uv_triple}/uv" "$target_dir/uv"
        chmod +x "$target_dir/uv"
        rm -rf "$target_dir/uv.tar.gz" "$target_dir/uv_extract"
    fi

    if [ ! -f "$target_dir/fnm" ]; then
        echo "📦 下载 fnm ($arch_name)..."
        download_with_fallback "$target_dir/fnm.zip" \
            "https://github.com/Schniz/fnm/releases/download/${FNM_RELEASE_TAG}/${fnm_pkg}" \
            "${GITHUB_RELEASE_MIRROR}/Schniz/fnm/releases/download/${FNM_RELEASE_TAG}/${fnm_pkg}"
        mkdir -p "$target_dir/fnm_extract"
        unzip -o "$target_dir/fnm.zip" -d "$target_dir/fnm_extract" > /dev/null
        cp "$target_dir/fnm_extract/fnm" "$target_dir/fnm"
        chmod +x "$target_dir/fnm"
        rm -rf "$target_dir/fnm.zip" "$target_dir/fnm_extract"
    fi
}

download_with_fallback() {
    local output_path=$1
    shift
    local urls=("$@")
    local url

    rm -f "$output_path"

    for url in "${urls[@]}"; do
        [ -z "$url" ] && continue
        echo "   ↳ 尝试下载: $url"
        if curl -fL --http1.1 --retry 5 --retry-delay 2 --retry-all-errors \
            --connect-timeout 20 --max-time 0 "$url" -o "$output_path"; then
            return 0
        fi
        echo "   ↳ 下载失败，尝试下一个源"
        rm -f "$output_path"
    done

    echo "❌ 所有下载源均失败: ${urls[*]}" >&2
    return 1
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
        cd "$FRONTEND_DIR"
        if [ ! -d "node_modules" ]; then
            echo "📦 正在安装前端依赖..."
            npm install
        fi
        npm run build
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
        BUILD_CMD="cargo build --release"
        ARCH_NAME=$(uname -m)
        BIN_SOURCE_PATH="target/release"

    elif [ "$TARGET_ALIAS" == "amd64" ]; then
        echo "🏗️  正在构建 AMD64 (x86_64)..."
        export CARGO_TARGET_DIR="target_amd64"
        TARGET_TRIPLE="x86_64-unknown-linux-musl"
        ARCH_NAME="x86_64"
        BIN_SOURCE_PATH="$CARGO_TARGET_DIR/$TARGET_TRIPLE/release"

        if ! command -v cross &> /dev/null; then echo "❌ 需要安装 cross 工具 (cargo install cross)"; exit 1; fi
        BUILD_CMD="cross build --release --target $TARGET_TRIPLE"

    elif [ "$TARGET_ALIAS" == "arm64" ]; then
        echo "🏗️  正在构建 ARM64 (aarch64)..."
        export CARGO_TARGET_DIR="target_arm64"
        TARGET_TRIPLE="aarch64-unknown-linux-musl"
        ARCH_NAME="aarch64"
        BIN_SOURCE_PATH="$CARGO_TARGET_DIR/$TARGET_TRIPLE/release"

        if ! command -v cross &> /dev/null; then echo "❌ 需要安装 cross 工具 (cargo install cross)"; exit 1; fi
        BUILD_CMD="cross build --release --target $TARGET_TRIPLE"

    elif [ "$TARGET_ALIAS" == "armv7" ]; then
        echo "🏗️  正在构建 ARMv7..."
        export CARGO_TARGET_DIR="target_armv7"
        TARGET_TRIPLE="armv7-unknown-linux-musleabihf"
        ARCH_NAME="armv7"
        BIN_SOURCE_PATH="$CARGO_TARGET_DIR/$TARGET_TRIPLE/release"

        if ! command -v cross &> /dev/null; then echo "❌ 需要安装 cross 工具 (cargo install cross)"; exit 1; fi
        BUILD_CMD="cross build --release --target $TARGET_TRIPLE"
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

    # 复制前端 (假设已构建)
    if [ -d "$FRONTEND_DIR/dist" ]; then
        cp -r "$FRONTEND_DIR/dist/." "$TEMP_DIR/web/"
    fi

    prepare_runtime_tools "$ARCH_NAME"
    cp "$TOOLS_CACHE_DIR/$ARCH_NAME/uv" "$TEMP_DIR/tools/uv"
    cp "$TOOLS_CACHE_DIR/$ARCH_NAME/fnm" "$TEMP_DIR/tools/fnm"

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
