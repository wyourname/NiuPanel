ARG RUST_IMAGE=rust:bookworm
FROM ${RUST_IMAGE}

ARG TARGETARCH
ARG NODE_VERSION=22.12.0

ENV DEBIAN_FRONTEND=noninteractive \
    npm_config_cache=/workspace/.cache/npm \
    UV_CACHE_DIR=/workspace/.cache/uv \
    FNM_DIR=/workspace/.cache/fnm \
    PATH=/usr/local/cargo/bin:/usr/local/node/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        git \
        procps \
        python3 \
        python3-pip \
        sqlite3 \
        xz-utils; \
    rm -rf /var/lib/apt/lists/*

RUN set -eux; \
    arch="${TARGETARCH:-amd64}"; \
    case "$arch" in \
        amd64) node_arch="x64" ;; \
        arm64) node_arch="arm64" ;; \
        arm) node_arch="armv7l" ;; \
        *) echo "Unsupported Node.js architecture: $arch"; exit 1 ;; \
    esac; \
    curl -fsSL "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-${node_arch}.tar.xz" -o /tmp/node.tar.xz; \
    mkdir -p /usr/local/node; \
    tar -xJf /tmp/node.tar.xz -C /usr/local/node --strip-components=1; \
    ln -s /usr/local/node/bin/node /usr/local/bin/node; \
    ln -s /usr/local/node/bin/npm /usr/local/bin/npm; \
    ln -s /usr/local/node/bin/npx /usr/local/bin/npx; \
    rm /tmp/node.tar.xz; \
    node --version; \
    npm --version

WORKDIR /workspace

CMD ["bash"]
