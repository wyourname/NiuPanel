#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPOSE_FILE="${COMPOSE_FILE:-compose.dev.yml}"
NIUPANEL_DOCKER_ARCH="${NIUPANEL_DOCKER_ARCH:-}"
NIUPANEL_DOCKER_PLATFORM=""

cd "$ROOT_DIR"

usage() {
    cat <<EOF
Usage: ./start.sh [command]

Commands:
  up        Start the Docker development environment (default)
  down      Stop the Docker development environment
  restart   Restart the Docker development environment
  logs      Follow Docker development logs
  config    Print the resolved Docker Compose config
  help      Show this help

Environment:
  RUST_IMAGE             Rust development image, default: rust:bookworm
  NODE_IMAGE             Node.js development image, default: node:22.23.1-bookworm-slim
  COMPOSE_FILE           Docker Compose file, default: compose.dev.yml
  NIUPANEL_DOCKER_ARCH   Docker architecture override: amd64, arm64, or arm
EOF
}

compose_cmd() {
    if docker compose version >/dev/null 2>&1; then
        DOCKER_COMPOSE=(docker compose)
    elif command -v docker-compose >/dev/null 2>&1; then
        DOCKER_COMPOSE=(docker-compose)
    else
        echo "Docker Compose is not available." >&2
        exit 1
    fi
}

detect_docker_arch() {
    local detected_arch="${NIUPANEL_DOCKER_ARCH:-}"

    if [ -z "$detected_arch" ]; then
        if ! detected_arch="$(docker info --format '{{.Architecture}}' 2>/dev/null)"; then
            echo "Unable to detect the Docker engine architecture." >&2
            exit 1
        fi
    fi

    case "$detected_arch" in
        amd64|x86_64)
            NIUPANEL_DOCKER_ARCH="amd64"
            NIUPANEL_DOCKER_PLATFORM="linux/amd64"
            ;;
        arm64|aarch64)
            NIUPANEL_DOCKER_ARCH="arm64"
            NIUPANEL_DOCKER_PLATFORM="linux/arm64"
            ;;
        arm|armv7|armv7l|armhf)
            NIUPANEL_DOCKER_ARCH="arm"
            NIUPANEL_DOCKER_PLATFORM="linux/arm/v7"
            ;;
        *)
            echo "Unsupported Docker architecture: $detected_arch" >&2
            echo "Supported architectures: x86_64/amd64, aarch64/arm64, armv7/armhf." >&2
            exit 1
            ;;
    esac

    export NIUPANEL_DOCKER_ARCH
}

prepare_compose() {
    if [ ! -f "$COMPOSE_FILE" ]; then
        echo "Compose file not found: $COMPOSE_FILE" >&2
        exit 1
    fi

    compose_cmd
    detect_docker_arch
}

run_compose() {
    prepare_compose
    "${DOCKER_COMPOSE[@]}" -f "$COMPOSE_FILE" "$@"
}

start_dev() {
    prepare_compose
    echo "Starting NiuPanel Docker development environment..."
    echo "Platform: $NIUPANEL_DOCKER_PLATFORM"
    echo "Frontend: http://localhost:7787"
    echo "Backend:  http://localhost:7788"
    "${DOCKER_COMPOSE[@]}" -f "$COMPOSE_FILE" up --build
}

stop_dev() {
    run_compose down --remove-orphans
}

restart_dev() {
    stop_dev
    start_dev
}

COMMAND="${1:-up}"

case "$COMMAND" in
    up|docker|dev)
        start_dev
        ;;
    down|stop)
        stop_dev
        ;;
    restart)
        restart_dev
        ;;
    logs)
        run_compose logs -f --tail=200
        ;;
    config)
        run_compose config
        ;;
    help|-h|--help)
        usage
        ;;
    *)
        echo "Unknown command: $COMMAND" >&2
        usage
        exit 1
        ;;
esac
