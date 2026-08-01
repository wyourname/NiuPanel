SKIPUNZIP=1

DATA_DIR="/data/adb/niupanel"
CHROOT_DIR="$DATA_DIR/debian"
REPO="wyourname/NiuPanel"
VERSION="v1.0.0"

ui_print "- 正在安装 NiuPanel 模块..."

unzip -o "$ZIPFILE" 'service.sh' -d $MODPATH >&2
unzip -o "$ZIPFILE" 'uninstall.sh' -d $MODPATH >&2
unzip -o "$ZIPFILE" 'module.prop' -d $MODPATH >&2
unzip -o "$ZIPFILE" 'tools/*' -d $MODPATH >&2
mkdir -p "$MODPATH/app/tools"

ARCH_RAW=$(getprop ro.product.cpu.abi)
case "$ARCH_RAW" in
    arm64-v8a|aarch64) ARCH="arm64" ;;
    armeabi-v7a|armv7*) ARCH="armv7" ;;
    *) abort "不支持的架构: $ARCH_RAW" ;;
esac
ui_print "- 检测到架构: $ARCH"

if [ -d "$MODPATH/tools/$ARCH" ]; then
    ui_print "- 复制离线工具 (uv) 到 app/tools..."
    cp -f "$MODPATH/tools/$ARCH/"* "$MODPATH/app/tools/" 2>/dev/null
    chmod +x "$MODPATH/app/tools/"* 2>/dev/null
    ui_print "- 离线工具就绪: $(ls "$MODPATH/app/tools/" 2>/dev/null | tr '\n' ' ')"
else
    ui_print "- 警告: 未找到 $ARCH 架构的离线工具，将依赖在线安装"
fi

mkdir -p "$CHROOT_DIR"
if [ ! -f "$CHROOT_DIR/bin/bash" ]; then
    ui_print "- 正在获取 Debian 基础环境 (下载约70MB)..."
    if [ "$ARCH" = "arm64" ]; then
        ROOTFS_URL="https://mirrors.tuna.tsinghua.edu.cn/lxc-images/images/debian/bookworm/arm64/default/20260419_19%3A38/rootfs.tar.xz"
    else
        ROOTFS_URL="https://mirrors.tuna.tsinghua.edu.cn/lxc-images/images/debian/bookworm/armhf/default/20260419_18%3A47/rootfs.tar.xz"
    fi
    curl -L -f --connect-timeout 20 --retry 3 "$ROOTFS_URL" -o "$MODPATH/rootfs.tar.xz"
    ui_print "- 正在解压 Debian 环境..."
    tar -xJf "$MODPATH/rootfs.tar.xz" -C "$CHROOT_DIR"
    rm "$MODPATH/rootfs.tar.xz"
fi

case "$ARCH" in
    arm64) ARCH_SUFFIX="aarch64" ;;
    armv7) ARCH_SUFFIX="armv7" ;;
esac

UPDATE_INDEX_URL="https://raw.githubusercontent.com/${REPO}/main/release/channels/stable.json"
UPDATE_INDEX="$MODPATH/update-index.json"
ui_print "- 正在读取稳定更新索引..."
curl -L -f --connect-timeout 20 --retry 3 "$UPDATE_INDEX_URL" -o "$UPDATE_INDEX" \
    || abort "无法获取稳定更新索引"

json_string() {
    printf '%s\n' "$1" | sed -n "s/^[[:space:]]*\"$2\"[[:space:]]*:[[:space:]]*\"\\([^\"]*\\)\"[,]*[[:space:]]*$/\\1/p" | head -n 1
}

json_number() {
    printf '%s\n' "$1" | sed -n "s/^[[:space:]]*\"$2\"[[:space:]]*:[[:space:]]*\\([0-9][0-9]*\\)[,]*[[:space:]]*$/\\1/p" | head -n 1
}

verify_download() {
    local label="$1"
    local url="$2"
    local expected_sha="$3"
    local expected_size="$4"
    local output="$5"
    curl -L -f --connect-timeout 20 --retry 3 "$url" -o "$output" \
        || abort "${label} 下载失败"
    local actual_size
    actual_size=$(wc -c < "$output" | tr -d '[:space:]')
    [ "$actual_size" = "$expected_size" ] \
        || abort "${label} 大小校验失败"
    command -v sha256sum >/dev/null 2>&1 \
        || abort "系统缺少 sha256sum，无法校验 ${label}"
    local actual_sha
    actual_sha=$(sha256sum "$output" | awk '{print $1}')
    [ "$actual_sha" = "$expected_sha" ] \
        || abort "${label} SHA-256 校验失败"
}

CORE_ASSET=$(sed -n "/\"${ARCH_SUFFIX}\"[[:space:]]*:[[:space:]]*{/,/^[[:space:]]*},[[:space:]]*$/p" "$UPDATE_INDEX")
CORE_URL=$(json_string "$CORE_ASSET" url)
CORE_SHA=$(json_string "$CORE_ASSET" sha256)
CORE_SIZE=$(json_number "$CORE_ASSET" size)
[ -n "$CORE_URL" ] && [ -n "$CORE_SHA" ] && [ -n "$CORE_SIZE" ] \
    || abort "稳定更新索引中缺少 ${ARCH_SUFFIX} Core 组件"
ui_print "- 正在获取 NiuPanel Core..."
verify_download "NiuPanel Core" "$CORE_URL" "$CORE_SHA" "$CORE_SIZE" "$MODPATH/app.tar.gz"
tar -xzf "$MODPATH/app.tar.gz" -C "$MODPATH/app/"
rm -f "$MODPATH/app.tar.gz"

WEB_ASSET=$(sed -n '/"asset"[[:space:]]*:[[:space:]]*{/,/^[[:space:]]*}[,]*[[:space:]]*$/p' "$UPDATE_INDEX")
WEB_URL=$(json_string "$WEB_ASSET" url)
WEB_SHA=$(json_string "$WEB_ASSET" sha256)
WEB_SIZE=$(json_number "$WEB_ASSET" size)
[ -n "$WEB_URL" ] && [ -n "$WEB_SHA" ] && [ -n "$WEB_SIZE" ] \
    || abort "稳定更新索引中缺少 Web 组件"
ui_print "- 正在获取与 Core 兼容的独立 Web UI..."
verify_download "NiuPanel Web UI" "$WEB_URL" "$WEB_SHA" "$WEB_SIZE" "$MODPATH/web.tar.gz"
mkdir -p "$MODPATH/app/web"
tar -xzf "$MODPATH/web.tar.gz" -C "$MODPATH/app/web/"
rm -f "$MODPATH/web.tar.gz" "$UPDATE_INDEX"
if [ ! -f "$MODPATH/app/web/release-manifest.json" ]; then
    abort "独立 Web UI 包无效"
fi

set_perm_recursive $MODPATH 0 0 0755 0755
set_perm $MODPATH/service.sh 0 0 0755

ui_print "- 安装成功！重启后自动初始化环境并运行。"
