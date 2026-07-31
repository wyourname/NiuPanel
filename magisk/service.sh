#!/system/bin/sh
MODDIR=${0%/*}
DATA_DIR="/data/adb/niupanel"
CHROOT_DIR="$DATA_DIR/debian"
REPO="wyourname/NiuPanel"
VERSION="v1.0.0"

until [ "$(getprop sys.boot_completed)" = "1" ]; do
    sleep 5
done

detect_arch() {
    local abi=$(getprop ro.product.cpu.abi)
    case "$abi" in
        arm64-v8a|aarch64) echo "arm64" ;;
        armeabi-v7a|armv7*) echo "armv7" ;;
        *) echo "arm64" ;;
    esac
}

ARCH=$(detect_arch)

mkdir -p "$CHROOT_DIR"
for dir in proc sys dev dev/pts; do
    mkdir -p "$CHROOT_DIR/$dir"
    mount -o bind /$dir $CHROOT_DIR/$dir 2>/dev/null
done

mkdir -p "$CHROOT_DIR/etc"

if [ -f "/etc/hosts" ] && [ -s "/etc/hosts" ]; then
    mount -o bind /etc/hosts "$CHROOT_DIR/etc/hosts" 2>/dev/null
fi

mkdir -p "$CHROOT_DIR/etc"
cat > "$CHROOT_DIR/etc/resolv.conf" <<RESOLV
nameserver 223.5.5.5
nameserver 119.29.29.29
nameserver 8.8.8.8
nameserver 1.1.1.1
RESOLV
chmod 644 "$CHROOT_DIR/etc/resolv.conf"

if [ -f "$CHROOT_DIR/etc/nsswitch.conf" ]; then
    sed -i 's/^hosts:.*/hosts:          files dns/' "$CHROOT_DIR/etc/nsswitch.conf" 2>/dev/null
else
    echo "hosts:          files dns" > "$CHROOT_DIR/etc/nsswitch.conf"
fi

APP_TOOLS_DIR="$MODDIR/app/tools"
mkdir -p "$APP_TOOLS_DIR"

if [ -d "$MODDIR/tools/$ARCH" ]; then
    cp -f "$MODDIR/tools/$ARCH/"* "$APP_TOOLS_DIR/" 2>/dev/null
    chmod +x "$APP_TOOLS_DIR/"* 2>/dev/null
fi

mkdir -p "$CHROOT_DIR/app"
mount -o bind "$MODDIR/app" "$CHROOT_DIR/app" 2>/dev/null

chroot "$CHROOT_DIR" /bin/bash <<'INITCHROOT'
    export HOME=/root
    export PATH=/usr/local/bin:/usr/bin:/bin:/sbin:/usr/sbin
    export DEBIAN_FRONTEND=noninteractive
    export UV_HTTP_TIMEOUT=120
    export UV_CACHE_DIR=/app/uv_cache

    echo "[NiuPanel] 确保 DNS 配置..."
    if [ ! -s "/etc/resolv.conf" ] || ! grep -q "nameserver" /etc/resolv.conf 2>/dev/null; then
        cat > /etc/resolv.conf <<DNS
nameserver 223.5.5.5
nameserver 119.29.29.29
nameserver 8.8.8.8
nameserver 1.1.1.1
DNS
        chmod 644 /etc/resolv.conf
    fi

    if [ -f "/etc/nsswitch.conf" ]; then
        sed -i 's/^hosts:.*/hosts:          files dns/' /etc/nsswitch.conf 2>/dev/null
    else
        echo "hosts:          files dns" > /etc/nsswitch.conf
    fi

    NEED_INIT=0
    if [ ! -f "/usr/bin/git" ]; then NEED_INIT=1; fi
    if [ ! -f "/usr/local/bin/uv" ] && [ ! -f "/usr/bin/uv" ]; then NEED_INIT=1; fi

    if [ "$NEED_INIT" = "1" ]; then
        echo "[NiuPanel] 正在初始化 Debian 基础环境..."
        sed -i 's|deb.debian.org|mirrors.ustc.edu.cn|g' /etc/apt/sources.list 2>/dev/null
        sed -i 's|security.debian.org|mirrors.ustc.edu.cn/debian-security|g' /etc/apt/sources.list 2>/dev/null
        apt-get update -y
        apt-get install -y curl ca-certificates unzip git python3 python3-venv libstdc++6 --no-install-recommends
        echo "[NiuPanel] Debian 基础环境安装完成"
    fi

    if [ ! -f "/usr/local/bin/uv" ] && [ ! -f "/usr/bin/uv" ]; then
        if [ -f "/app/tools/uv" ]; then
            echo "[NiuPanel] 植入离线 uv..."
            cp -f /app/tools/uv /usr/local/bin/uv
            chmod +x /usr/local/bin/uv
            if [ -f "/app/tools/uvx" ]; then
                cp -f /app/tools/uvx /usr/local/bin/uvx
                chmod +x /usr/local/bin/uvx
            fi
            echo "[NiuPanel] uv 植入成功: $(/usr/local/bin/uv --version 2>/dev/null || echo 'unknown')"
        else
            echo "[NiuPanel] 离线 uv 不存在，尝试在线安装..."
            curl -LsSf https://astral.sh/uv/install.sh | env UV_INSTALL_DIR=/usr/local/bin sh || true
        fi
    fi

    export PNPM_NODE_DIST_MIRROR="https://mirrors.ustc.edu.cn/node/"
    export NPM_CONFIG_REGISTRY="https://registry.npmmirror.com/"
    export PATH="/usr/local/bin:$PATH"

    cd /app
    [ -L web ] || ln -snf /app/web web 2>/dev/null

    export RUST_LOG=info
    export HOST=0.0.0.0
    export PORT=7788

    echo "[NiuPanel] === 环境检查 ==="
    echo "  DNS 配置: $(cat /etc/resolv.conf | grep nameserver | head -2 | tr '\n' ' ')"
    echo "  DNS 测试: $(getent hosts baidu.com 2>/dev/null | head -1 || echo 'DNS 解析异常')"
    echo "  git:   $(git --version 2>/dev/null || echo '未安装')"
    echo "  python3: $(python3 --version 2>/dev/null || echo '未安装')"
    echo "  uv:    $(command -v uv >/dev/null 2>&1 && uv --version 2>/dev/null || echo '未找到')"
    echo "  pnpm:  由 NiuPanel 首次使用时自动准备"

    if ! pgrep niupanel > /dev/null; then
        echo "[NiuPanel] 启动中..."
        nohup ./niupanel >> /app/android_server.log 2>&1 &
        sleep 1
        if pgrep niupanel > /dev/null; then
            echo "[NiuPanel] 启动成功 (PID: $(pgrep niupanel))"
        else
            echo "[NiuPanel] 启动失败，请检查 /app/android_server.log"
        fi
    else
        echo "[NiuPanel] 已在运行 (PID: $(pgrep niupanel))"
    fi
INITCHROOT
