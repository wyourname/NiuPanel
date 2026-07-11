#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPOSE_FILE="${COMPOSE_FILE:-compose.dev.yml}"

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
  RUST_IMAGE     Rust development image, default: rust:bookworm
  COMPOSE_FILE   Docker Compose file, default: compose.dev.yml
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

run_compose() {
    if [ ! -f "$COMPOSE_FILE" ]; then
        echo "Compose file not found: $COMPOSE_FILE" >&2
        exit 1
    fi

    compose_cmd
    "${DOCKER_COMPOSE[@]}" -f "$COMPOSE_FILE" "$@"
}

start_dev() {
    echo "Starting NiuPanel Docker development environment..."
    echo "Frontend: http://localhost:7787"
    echo "Backend:  http://localhost:7788"
    run_compose up --build
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
