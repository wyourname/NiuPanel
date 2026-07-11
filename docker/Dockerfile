# 使用中科大的 Debian 基础镜像
FROM debian:bookworm-slim

ARG TARGETARCH

WORKDIR /app

# 设置时区和环境变量
ENV TZ=Asia/Shanghai \
    DEBIAN_FRONTEND=noninteractive \
    UV_INDEX_URL=https://pypi.mirrors.ustc.edu.cn/simple/ \
    UV_PIP_INDEX_URL=https://pypi.mirrors.ustc.edu.cn/simple/ \
    UV_TOOL_DIR=/app/data/uv/tool \
    UV_CACHE_DIR=/app/data/uv/cache \
    UV_PYTHON_INSTALL_DIR=/app/data/uv/python \
    FNM_DIR=/app/data/fnm \
    FNM_NODE_DIST_MIRROR=https://mirrors.ustc.edu.cn/node/
    # removed FNM_DIR from PATH as fnm binary will now strictly be in /usr/local/bin

RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone

# 1. 系统依赖安装 (合并所有 apt 操作以减小镜像体积)
RUN set -ex; \
    # 修改为中科大源 (HTTP)
    rm -f /etc/apt/sources.list.d/*; \
    echo "deb http://mirrors.ustc.edu.cn/debian bookworm main contrib non-free non-free-firmware" > /etc/apt/sources.list; \
    echo "deb http://mirrors.ustc.edu.cn/debian bookworm-updates main contrib non-free non-free-firmware" >> /etc/apt/sources.list; \
    echo "deb http://mirrors.ustc.edu.cn/debian-security bookworm-security main contrib non-free non-free-firmware" >> /etc/apt/sources.list; \
    # 安装依赖（移除 xz-utils，已不再需要解压 node tarball）
    apt-get update; \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        gnupg2 \
        procps \
        git \
        python3 \
        python3-pip \
        libatomic1; \
    # 清理缓存
    apt-get clean;

# 2. 复制并解压应用
# 注意：COPY 指令会增加一层镜像体积，即使后面删除了文件。
# 如果想进一步优化体积，建议在 docker build 前只拷贝对应架构的包，或使用 ARG 传递文件名。
COPY niupanel_linux_*.tar.gz /tmp/app_pkgs/

RUN set -ex; \
    FILE_ARCH=""; \
    DETECTED_ARCH="${TARGETARCH:-$(dpkg --print-architecture)}"; \
    case "${DETECTED_ARCH}" in \
        "amd64"|"x86_64") FILE_ARCH="x86_64" ;; \
        "arm64"|"aarch64") FILE_ARCH="aarch64" ;; \
        "arm"|"armv7"|"armhf") FILE_ARCH="armv7" ;; \
        *) echo "Unsupported app architecture: ${DETECTED_ARCH}"; exit 1 ;; \
    esac; \
     \
    echo "Extracting application for: ${FILE_ARCH} ..."; \
    TAR_FILE=$(find /tmp/app_pkgs/ -name "niupanel_linux_${FILE_ARCH}.tar.gz" | head -n 1); \
    if [ -z "$TAR_FILE" ]; then \
        echo "Error: Could not find package for ${FILE_ARCH}"; \
        ls -l /tmp/app_pkgs/; \
        exit 1; \
    fi; \
    tar -xzf "$TAR_FILE" -C /app/; \
    chmod +x /app/niupanel /app/niupanel-launcher; \
    rm -rf /tmp/app_pkgs

# 3. 暴露端口与启动
EXPOSE 7788
ENV SYSTEM_DIR=/app/data/system \
    NIUPANEL_SYSTEM_DIR=/app/data/system \
    BUNDLED_WEB_DIR=/app/web \
    PLUGIN_SIGNATURE_REQUIRED=true \
    NIUPANEL_BUNDLED_CORE=/app/niupanel
CMD ["./niupanel-launcher"]
