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
    PNPM_NODE_DIST_MIRROR=https://mirrors.ustc.edu.cn/node/ \
    NPM_CONFIG_REGISTRY=https://registry.npmmirror.com/

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
        libatomic1 \
        libstdc++6; \
    # 清理缓存
    apt-get clean;

# 2. 只复制当前目标架构的应用包。
# `packages/{amd64,arm64,arm}/niupanel.tar.gz` 由本地构建脚本或 CI 准备，
# 避免把另外两个架构的归档写入镜像层。
COPY packages/${TARGETARCH}/niupanel.tar.gz /tmp/niupanel.tar.gz

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
    tar -xzf /tmp/niupanel.tar.gz -C /app/; \
    chmod +x /app/niupanel /app/niupanel-launcher /app/tools/uv /app/tools/pnpm; \
    test ! -e /app/tools/fnm; \
    BUNDLED_PNPM_VERSION=$(/app/tools/pnpm --version); \
    if [ "$BUNDLED_PNPM_VERSION" != "11.18.0" ]; then \
        echo "Unexpected bundled pnpm version: $BUNDLED_PNPM_VERSION"; \
        exit 1; \
    fi; \
    /app/tools/uv --version; \
    rm -f /tmp/niupanel.tar.gz

ARG NIUPANEL_ENVIRONMENT_VERSION=unknown
ARG NIUPANEL_CORE_VERSION=unknown

LABEL org.opencontainers.image.title="NiuPanel" \
      org.opencontainers.image.version="${NIUPANEL_ENVIRONMENT_VERSION}" \
      io.niupanel.core.version="${NIUPANEL_CORE_VERSION}"

# 3. 暴露端口与启动
EXPOSE 7788
ENV SYSTEM_DIR=/app/data/system \
    NIUPANEL_SYSTEM_DIR=/app/data/system \
    BUNDLED_WEB_DIR=/app/web \
    PLUGIN_SIGNATURE_REQUIRED=true \
    NIUPANEL_BUNDLED_CORE=/app/niupanel
CMD ["./niupanel-launcher"]
