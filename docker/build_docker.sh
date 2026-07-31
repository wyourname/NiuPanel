#!/bin/bash
set -e

# --- Configuration ---
VERSION="${DOCKER_IMAGE_VERSION:-$(tr -d '[:space:]' < VERSION)}"
VERSION="${VERSION#v}"
CORE_VERSION=$(sed -n '/^\[package\]/,/^\[/{s/^version = "\([^"]*\)"/\1/p}' ../niupanel/Cargo.toml | head -n 1)
IMAGE_NAME="niupanel"
DOCKER_USERNAME="${DOCKER_HUB_USERNAME:-wyourname}"
BASE_IMAGE="${DOCKER_USERNAME}/${IMAGE_NAME}"

# Architectures: Key=PackageSuffix, Value=DockerPlatform
declare -A ARCH_MAP=(
    ["x86_64"]="linux/amd64"
    ["aarch64"]="linux/arm64"
    ["armv7"]="linux/arm/v7"
)

# --- Helper Functions ---

log() { echo -e "[\033[1;32m$(date +'%H:%M:%S')\033[0m] $*"; }
err() { echo -e "[\033[1;31mERROR\033[0m] $*" >&2; }

cleanup() {
    log "🧹 开始清理本地资源..."

    # Clean manifests
    for tag in "latest" "${VERSION}"; do
        docker manifest rm "${BASE_IMAGE}:${tag}" 2>/dev/null || true
    done

    # Clean images
    for suffix in "${!ARCH_MAP[@]}"; do
        local docker_arch=$(echo "${ARCH_MAP[$suffix]}" | cut -d'/' -f2) # Extract amd64/arm64/arm
        local variant=$(echo "${ARCH_MAP[$suffix]}" | cut -d'/' -f3)     # Extract v7 if exists

        # Normalize arch name for tagging (amd64, arm64, armv7)
        local tag_arch="${docker_arch}"
        [[ -n "$variant" ]] && tag_arch="${docker_arch}${variant}"

        docker rmi "${IMAGE_NAME}:${VERSION}-${tag_arch}" \
                   "${BASE_IMAGE}:${VERSION}-${tag_arch}" 2>/dev/null || true
    done

    log "🗑️  清理悬空镜像..."
    docker image prune -f
    log "✨ 清理完成。"
}

build_and_push() {
    local pkg_suffix=$1
    local platform=$2

    # Normalize tag architecture: amd64, arm64, armv7
    local tag_arch=$(echo "$platform" | awk -F/ '{if ($3) print $2$3; else print $2}')
    local tar_file="niupanel_linux_${pkg_suffix}.tar.gz"
    local context_arch=$(echo "$platform" | awk -F/ '{print $2}')

    if [ ! -f "$tar_file" ]; then
        log "⚠️ 跳过 ${pkg_suffix}: 未找到文件 ${tar_file}"
        return 1
    fi

    log "📦 构建: ${pkg_suffix} -> ${platform}"
    mkdir -p "packages/${context_arch}"
    cp "$tar_file" "packages/${context_arch}/niupanel.tar.gz"

    # Build
    docker build --platform "${platform}" \
        --provenance=false \
        --pull \
        --build-arg NIUPANEL_ENVIRONMENT_VERSION="${VERSION}" \
        --build-arg NIUPANEL_CORE_VERSION="${CORE_VERSION}" \
        -t "${IMAGE_NAME}:${VERSION}-${tag_arch}" .

    # Tag & Push
    local remote_version="${BASE_IMAGE}:${VERSION}-${tag_arch}"

    log "🚀 推送: ${remote_version}"
    local retries=0
    local max_retries=10

    # Retry logic for push (version)
    retries=0
    until [ $retries -ge $max_retries ]
    do
        if docker push "${remote_version}"; then
            break
        fi
        retries=$((retries+1))
        log "⚠️ Push failed. Retrying ($retries/$max_retries) in 5s..."
        sleep 5
    done

    if [ $retries -eq $max_retries ]; then
        err "❌ Failed to push ${remote_version} after $max_retries attempts."
        return 1
    fi

    # Store for manifest
    BUILT_TAGS+=("${tag_arch}")
}

create_manifest() {
    local target_tag=$1
    log "📝 创建 Manifest: ${target_tag}"

    # Prepare source tags list
    local src_tags=()
    for arch in "${BUILT_TAGS[@]}"; do
        src_tags+=("${BASE_IMAGE}:${VERSION}-${arch}")
    done

    docker manifest rm "${target_tag}" 2>/dev/null || true

    # Retry logic for manifest create
    local retries=0
    local max_retries=10
    until [ $retries -ge $max_retries ]
    do
        if docker manifest create "${target_tag}" --amend "${src_tags[@]}"; then
            break
        fi
        retries=$((retries+1))
        log "⚠️ Manifest create failed. Retrying ($retries/$max_retries) in 5s..."
        sleep 5
    done

    if [ $retries -eq $max_retries ]; then
        err "❌ Failed to create manifest after $max_retries attempts."
        return 1
    fi

    # Annotate
    for arch in "${BUILT_TAGS[@]}"; do
        local full_image="${BASE_IMAGE}:${VERSION}-${arch}"
        if [[ "$arch" == "armv7" ]]; then
            docker manifest annotate "${target_tag}" "${full_image}" --os linux --arch arm --variant v7
        elif [[ "$arch" == "arm64" ]]; then
            docker manifest annotate "${target_tag}" "${full_image}" --os linux --arch arm64
        else
            docker manifest annotate "${target_tag}" "${full_image}" --os linux --arch amd64
        fi
    done

    log "☁️  推送 Manifest: ${target_tag}"

    # Retry logic for manifest push
    retries=0
    until [ $retries -ge $max_retries ]
    do
        if docker manifest push "${target_tag}"; then
            break
        fi
        retries=$((retries+1))
        log "⚠️ Manifest push failed. Retrying ($retries/$max_retries) in 5s..."
        sleep 5
    done

    if [ $retries -eq $max_retries ]; then
        err "❌ Failed to push manifest after $max_retries attempts."
        return 1
    fi
}

# --- Main Execution ---

# Setup trap
trap 'err "脚本执行出错，开始清理..."; cleanup; exit 1' ERR

if [ ! -f "Dockerfile" ]; then
    err "未找到 Dockerfile"
    exit 1
fi

log "🚀 开始构建流程 | 版本: ${VERSION} | 用户: ${DOCKER_USERNAME}"

log "🔧 正在配置 multi-arch 构建环境..."
docker run --privileged --rm tonistiigi/binfmt --install all

BUILT_TAGS=()

# Iterate and Build
for suffix in "${!ARCH_MAP[@]}"; do
    build_and_push "$suffix" "${ARCH_MAP[$suffix]}"
done

# Create Manifests if any images were built
if [ ${#BUILT_TAGS[@]} -gt 0 ]; then
    create_manifest "${BASE_IMAGE}:latest"
    create_manifest "${BASE_IMAGE}:${VERSION}"
    log "✅ 所有任务完成！"
    cleanup
else
    log "⚠️  没有构建任何镜像。"
fi
