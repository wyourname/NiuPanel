#!/bin/bash
set -e

# 配置
REPO="wyourname/NiuPanel"
VERSION="v2.1.4"
TOOLS_DIR="magisk/tools"

# 下载并准备工具的函数
prepare_tool() {
    local arch=$1
    local uv_triple=$2
    local fnm_pkg=$3
    local target_dir="$TOOLS_DIR/$arch"

    mkdir -p "$target_dir"

    # 1. 获取 UV
    if [ ! -f "$target_dir/uv" ]; then
        echo "🔍 获取 UV ($arch)..."
        curl -L "https://github.com/astral-sh/uv/releases/latest/download/uv-${uv_triple}.tar.gz" -o "$target_dir/uv.tar.gz"
        tar -xzf "$target_dir/uv.tar.gz" -C "$target_dir" --strip-components=1
        rm "$target_dir/uv.tar.gz"
    fi

    # 2. 获取 FNM
    if [ ! -f "$target_dir/fnm" ]; then
        echo "🔍 获取 FNM ($arch)..."
        if curl -L -f "https://github.com/Schniz/fnm/releases/latest/download/${fnm_pkg}" -o "$target_dir/fnm.zip"; then
            unzip -o "$target_dir/fnm.zip" -d "$target_dir"
            rm "$target_dir/fnm.zip"
            chmod +x "$target_dir/fnm"
        fi
    fi
}

echo "📦 正在准备通用离线工具链 (包含 arm64 和 armv7)..."
prepare_tool "arm64" "aarch64-unknown-linux-gnu" "fnm-arm64.zip"
prepare_tool "armv7" "armv7-unknown-linux-gnueabihf" "fnm-arm32.zip"

echo "🚀 开始打包通用 Magisk 模块..."
TMP_DIR="magisk_tmp"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

# 拷贝基础模板 (包含刚才下载的 tools/arm64 和 tools/armv7)
cp -r magisk/* "$TMP_DIR/"

ZIP_NAME="niupanel_universal_offline_tools.zip"
(cd "$TMP_DIR" && zip -q -r "../$ZIP_NAME" .)

rm -rf "$TMP_DIR"
echo "✅ 通用模块打包完成: $ZIP_NAME"
