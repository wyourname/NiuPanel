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
    ui_print "- 复制离线工具 (uv, fnm) 到 app/tools..."
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

FILE_NAME="niupanel_linux_${ARCH_SUFFIX}.tar.gz"
URL="https://github.com/${REPO}/releases/latest/download/${FILE_NAME}"
ui_print "- 正在获取 NiuPanel 二进制..."
curl -L -f --connect-timeout 20 --retry 3 "$URL" -o "$MODPATH/app.tar.gz"
if [ -f "$MODPATH/app.tar.gz" ]; then
    tar -xzf "$MODPATH/app.tar.gz" -C "$MODPATH/app/"
    rm "$MODPATH/app.tar.gz"
fi

set_perm_recursive $MODPATH 0 0 0755 0755
set_perm $MODPATH/service.sh 0 0 0755

ui_print "- 安装成功！重启后自动初始化环境并运行。"
