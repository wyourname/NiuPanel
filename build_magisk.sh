#!/bin/bash
set -e

# 配置
REPO="wyourname/NiuPanel"
VERSION="v2.1.4"
TOOLS_DIR="magisk/tools"

prepare_tool() {
    local arch=$1
    local target_dir="$TOOLS_DIR/$arch"
    scripts/prepare-runtime-tools.sh "$arch" "$target_dir"
}

echo "📦 正在准备通用离线工具链 (uv + pnpm，包含 arm64 和 armv7)..."
prepare_tool "arm64"
prepare_tool "armv7"

echo "🚀 开始打包通用 Magisk 模块..."
TMP_DIR="magisk_tmp"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

# 拷贝基础模板 (包含刚才下载的 tools/arm64 和 tools/armv7)
cp -r magisk/* "$TMP_DIR/"
# 旧缓存可能仍含 fnm，新包只分发 uv + pnpm。
rm -f "$TMP_DIR/tools/arm64/fnm" "$TMP_DIR/tools/armv7/fnm"
rm -f "$TMP_DIR/tools/arm64/.uv-version" "$TMP_DIR/tools/arm64/.pnpm-version"
rm -f "$TMP_DIR/tools/armv7/.uv-version" "$TMP_DIR/tools/armv7/.pnpm-version"
test -x "$TMP_DIR/tools/arm64/pnpm"
test -x "$TMP_DIR/tools/armv7/pnpm"

ZIP_NAME="niupanel_universal_offline_tools.zip"
(cd "$TMP_DIR" && zip -q -r "../$ZIP_NAME" .)

rm -rf "$TMP_DIR"
echo "✅ 通用模块打包完成: $ZIP_NAME"
