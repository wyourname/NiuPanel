ARG NODE_IMAGE=node:22.23.1-bookworm-slim
ARG RUST_IMAGE=rust:bookworm

FROM ${NODE_IMAGE} AS node-toolchain

FROM ${RUST_IMAGE}

ARG TARGETARCH
ARG NIUPANEL_TARGET_ARCH

ENV DEBIAN_FRONTEND=noninteractive \
    COREPACK_NPM_REGISTRY=https://registry.npmmirror.com \
    NIUPANEL_TOOLS_DIR=/opt/niupanel/tools \
    PNPM_HOME=/workspace/.cache/pnpm \
    npm_config_cache=/workspace/.cache/npm \
    UV_CACHE_DIR=/workspace/.cache/uv \
    PATH=/workspace/.cache/pnpm:/usr/local/cargo/bin:/usr/local/node/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        git \
        procps \
        python3 \
        python3-pip \
        libatomic1 \
        libstdc++6 \
        sqlite3; \
    rm -rf /var/lib/apt/lists/*

COPY --from=node-toolchain /usr/local/ /usr/local/

RUN rustup component add rustfmt clippy

COPY scripts/prepare-runtime-tools.sh /usr/local/libexec/niupanel-prepare-runtime-tools

RUN set -eux; \
    chmod 0755 /usr/local/libexec/niupanel-prepare-runtime-tools; \
    runtime_arch="${NIUPANEL_TARGET_ARCH:-${TARGETARCH:-$(dpkg --print-architecture)}}"; \
    NIUPANEL_RUNTIME_TOOLS=uv \
        /usr/local/libexec/niupanel-prepare-runtime-tools "$runtime_arch" "$NIUPANEL_TOOLS_DIR"; \
    corepack enable pnpm; \
    corepack prepare pnpm@12.9.1 --activate; \
    ln -s /usr/local/bin/pnpm "$NIUPANEL_TOOLS_DIR/pnpm"; \
    test ! -e "$NIUPANEL_TOOLS_DIR/fnm"; \
    test "$(node --version)" = "v22.23.1"; \
    test "$(pnpm --version)" = "12.9.1"; \
    "$NIUPANEL_TOOLS_DIR/uv" --version

WORKDIR /workspace

CMD ["bash"]
