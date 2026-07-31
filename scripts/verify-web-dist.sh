#!/usr/bin/env bash
set -Eeuo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"

PORT=8080
DIST_DIR="$REPO_ROOT/niupanelweb/dist"
KEEP_SERVING=false
SERVER_PID=""
LOG_FILE="$(mktemp "${TMPDIR:-/tmp}/niupanel-web-dist.XXXXXX.log")"

usage() {
    cat <<'EOF'
Usage: scripts/verify-web-dist.sh [options]

Validate the built Web assets, start a loopback-only static server, and run HTTP
smoke checks. The server exits after validation unless --serve is specified.

Options:
  --dist DIR    Web dist directory (default: niupanelweb/dist)
  --port PORT   Local HTTP port (default: 8080)
  --serve       Keep the server running after checks; stop it with Ctrl+C
  -h, --help    Show this help
EOF
}

log() {
    printf '[web-dist] %s\n' "$*"
}

fail() {
    printf '[web-dist] ERROR: %s\n' "$*" >&2
    exit 1
}

cleanup() {
    if [[ -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
    rm -f -- "$LOG_FILE"
}
trap cleanup EXIT INT TERM

while (($# > 0)); do
    case "$1" in
        --dist)
            (($# >= 2)) || fail "--dist 缺少目录参数"
            DIST_DIR="$2"
            shift 2
            ;;
        --port)
            (($# >= 2)) || fail "--port 缺少端口参数"
            PORT="$2"
            shift 2
            ;;
        --serve)
            KEEP_SERVING=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            fail "未知参数: $1"
            ;;
    esac
done

[[ "$PORT" =~ ^[0-9]+$ ]] || fail "端口必须是数字: $PORT"
((PORT >= 1 && PORT <= 65535)) || fail "端口必须在 1..65535 范围内"
command -v python3 >/dev/null 2>&1 || fail "未找到 python3"
command -v curl >/dev/null 2>&1 || fail "未找到 curl"
[[ -d "$DIST_DIR" ]] || fail "构建产物不存在: $DIST_DIR；请先运行 cd niupanelweb && pnpm run build"

required_files=(index.html favicon.png)
required_dirs=(assets monacoeditorwork)
for relative_path in "${required_files[@]}"; do
    [[ -s "$DIST_DIR/$relative_path" ]] || fail "关键文件缺失或为空: $relative_path"
done
for relative_path in "${required_dirs[@]}"; do
    [[ -d "$DIST_DIR/$relative_path" ]] || fail "关键目录缺失: $relative_path/"
done

mapfile -t js_files < <(find "$DIST_DIR/assets" -maxdepth 1 -type f -name '*.js' -printf '%f\n' | sort)
mapfile -t css_files < <(find "$DIST_DIR/assets" -maxdepth 1 -type f -name '*.css' -printf '%f\n' | sort)
((${#js_files[@]} > 0)) || fail "assets 中没有 JavaScript 产物"
((${#css_files[@]} > 0)) || fail "assets 中没有 CSS 产物"

total_files="$(find "$DIST_DIR" -type f | wc -l | tr -d ' ')"
total_size="$(du -sh "$DIST_DIR" | cut -f1)"
log "静态产物检查通过：$total_files 个文件，$total_size，JS ${#js_files[@]} 个，CSS ${#css_files[@]} 个"

python3 -m http.server "$PORT" \
    --bind 127.0.0.1 \
    --directory "$DIST_DIR" \
    >"$LOG_FILE" 2>&1 &
SERVER_PID=$!

base_url="http://127.0.0.1:$PORT"
ready=false
for _ in {1..30}; do
    if curl --fail --silent --output /dev/null --connect-timeout 1 "$base_url/"; then
        ready=true
        break
    fi
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        break
    fi
    sleep 0.1
done

if [[ "$ready" != true ]]; then
    sed -n '1,80p' "$LOG_FILE" >&2 || true
    fail "本地静态服务器启动失败，端口可能已占用: $PORT"
fi

check_url() {
    local path="$1"
    curl \
        --fail \
        --silent \
        --show-error \
        --output /dev/null \
        --connect-timeout 3 \
        --max-time 10 \
        "$base_url/$path" ||
        fail "HTTP 检查失败: /$path"
}

check_url ""
check_url "favicon.png"
check_url "assets/${js_files[0]}"
check_url "assets/${css_files[0]}"

index_bytes="$(curl --fail --silent --show-error --max-time 10 "$base_url/" | wc -c | tr -d ' ')"
log "HTTP 冒烟检查通过：http://127.0.0.1:$PORT（index.html ${index_bytes} bytes）"

if [[ "$KEEP_SERVING" == true ]]; then
    log "服务器保持运行，按 Ctrl+C 停止"
    wait "$SERVER_PID"
fi
