#!/bin/bash
set -e

# ============================================================
# NiuPanel 构建产物测试脚本 (product.sh)
# 用途：快速验证前端构建产物是否正常工作
# 使用方法: ./product.sh [端口号]
# 默认端口: 8080
# ============================================================

# --- 配置 ---
PORT="${1:-8080}"
DIST_DIR="./niupanelweb/dist"
LOG_FILE="/tmp/niupanel-test-server.log"
PID_FILE="/tmp/niupanel-test-server.pid"

# --- 颜色定义 ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# --- 辅助函数 ---
log_info() { echo -e "[${GREEN}INFO${NC}] $*"; }
log_warn() { echo -e "[${YELLOW}WARN${NC}] $*" >&2; }
log_error() { echo -e "[${RED}ERROR${NC}] $*" >&2; }
log_step() { echo -e "\n${BLUE}▶ $*${NC}"; }

check_command() {
    if ! command -v "$1" &>/dev/null; then
        log_error "未找到命令: $1"
        exit 1
    fi
}

cleanup() {
    log_step "清理资源..."

    if [ -f "$PID_FILE" ]; then
        local pid=$(cat "$PID_FILE")
        if kill -0 "$pid" 2>/dev/null; then
            log_info "停止服务器 (PID: $pid)..."
            kill "$pid" 2>/dev/null || true
            sleep 1
        fi
        rm -f "$PID_FILE"
    fi

    # 清理临时文件
    rm -f "$LOG_FILE"
}

# --- 主函数 ---

main() {
    echo ""
    echo "╔══════════════════════════════════════════════╗"
    echo "║     NiuPanel 构建产物测试工具 v1.0           ║"
    echo "╚══════════════════════════════════════════════╝"
    echo ""

    # 设置退出时自动清理
    trap cleanup EXIT

    # --- Step 1: 检查必要命令 ---
    log_step "检查环境依赖..."
    check_command "python3"
    check_command "curl"

    # --- Step 2: 检查构建产物 ---
    log_step "检查构建产物..."

    if [ ! -d "$DIST_DIR" ]; then
        log_error "构建产物目录不存在: $DIST_DIR"
        log_info "请先运行: cd niupanelweb && npm run build"
        exit 1
    fi

    # 检查关键文件
    local required_files=("index.html" "favicon.png")
    local required_dirs=("assets" "monacoeditorwork")

    for file in "${required_files[@]}"; do
        if [ -f "$DIST_DIR/$file" ]; then
            log_info "✓ 文件存在: $file ($(du -h "$DIST_DIR/$file" | cut -f1))"
        else
            log_error "✗ 文件缺失: $file"
        fi
    done

    for dir in "${required_dirs[@]}"; do
        if [ -d "$DIST_DIR/$dir" ]; then
            local file_count=$(find "$DIST_DIR/$dir" -type f | wc -l)
            local dir_size=$(du -sh "$DIST_DIR/$dir" | cut -f1)
            log_info "✓ 目录存在: $dir/ ($file_count 个文件, $dir_size)"
        else
            log_error "✗ 目录缺失: $dir/"
        fi
    done

    # 统计总大小
    local total_size=$(du -sh "$DIST_DIR" | cut -f1)
    local total_files=$(find "$DIST_DIR" -type f | wc -l)
    log_info "📦 构建产物总大小: $total_size ($total_files 个文件)"

    # --- Step 3: 分析资源文件 ---
    log_step "分析静态资源..."

    # JS/CSS 文件统计
    local js_files=$(find "$DIST_DIR/assets" -name "*.js" 2>/dev/null | wc -l)
    local css_files=$(find "$DIST_DIR/assets" -name "*.css" 2>/dev/null | wc -l)
    local js_size=$(du -sh $(find "$DIST_DIR/assets" -name "*.js" -exec dirname {} \; | sort -u) 2>/dev/null | tail -1 | cut -f1 || echo "0B")
    local css_size=$(du -sh $(find "$DIST_DIR/assets" -name "*.css" -exec dirname {} \; | sort -u) 2>/dev/null | tail -1 | cut -f1 || echo "0B")

    log_info "📄 JavaScript 文件: $js_files 个"
    log_info "🎨 CSS 文件: $css_files 个"

    # 检查是否有 vendor chunks（优化后的产物）
    if ls "$DIST_DIR/assets"/vendor-*.js 1>/dev/null 2>&1; then
        log_info "✓ 检测到代码分割 (vendor chunks)"
    fi

    # --- Step 4: 启动本地服务器 ---
    log_step "启动本地测试服务器..."

    # 检查端口是否被占用
    if lsof -i :"$PORT" >/dev/null 2>&1; then
        log_warn "端口 $PORT 已被占用，尝试使用端口 $((PORT+1))..."
        PORT=$((PORT+1))
    fi

    # 启动 Python HTTP 服务器
    cd "$DIST_DIR"
    python3 -m http.server "$PORT" > "$LOG_FILE" 2>&1 &
    local server_pid=$!
    cd - > /dev/null

    echo "$server_pid" > "$PID_FILE"

    # 等待服务器启动
    sleep 1

    if ! kill -0 "$server_pid" 2>/dev/null; then
        log_error "服务器启动失败！"
        cat "$LOG_FILE" 2>/dev/null || true
        exit 1
    fi

    log_info "✓ 服务器已启动 (PID: $server_pid)"
    log_info "🌐 访问地址: http://localhost:$PORT"

    # --- Step 5: HTTP 健康检查 ---
    log_step "执行 HTTP 健康检查..."

    local base_url="http://localhost:$PORT"
    local all_passed=true

    # 检查主页
    log_info "检查主页 (index.html)..."
    local http_code=$(curl -s -o /dev/null -w "%{http_code}" "$base_url/" --connect-timeout 5)
    if [ "$http_code" = "200" ]; then
        log_info "✓ 主页可访问 (HTTP $http_code)"
    else
        log_error "✗ 主页返回错误 (HTTP $http_code)"
        all_passed=false
    fi

    # 检查 favicon
    log_info "检查图标 (favicon.png)..."
    http_code=$(curl -s -o /dev/null -w "%{http_code}" "$base_url/favicon.png" --connect-timeout 5)
    if [ "$http_code" = "200" ]; then
        log_info "✓ 图标可访问 (HTTP $http_code)"
    else
        log_warn "⚠ 图标无法访问 (HTTP $http_code)"
    fi

    # 检查关键 JS 资源
    log_info "检查 JavaScript 资源..."
    local first_js=$(ls "$DIST_DIR/assets"/*.js 2>/dev/null | head -1 | xargs basename 2>/dev/null)
    if [ -n "$first_js" ]; then
        http_code=$(curl -s -o /dev/null -w "%{http_code}" "$base_url/assets/$first_js" --connect-timeout 5)
        if [ "$http_code" = "200" ]; then
            log_info "✓ JS 资源可访问: $first_js"
        else
            log_error "✗ JS 资源不可访问 (HTTP $http_code)"
            all_passed=false
        fi
    fi

    # 检查 CSS 资源
    log_info "检查 CSS 资源..."
    local first_css=$(ls "$DIST_DIR/assets"/*.css 2>/dev/null | head -1 | xargs basename 2>/dev/null)
    if [ -n "$first_css" ]; then
        http_code=$(curl -s -o /dev/null -w "%{http_code}" "$base_url/assets/$first_css" --connect-timeout 5)
        if [ "$http_code" = "200" ]; then
            log_info "✓ CSS 资源可访问: $first_css"
        else
            log_error "✗ CSS 资源不可访问 (HTTP $http_code)"
            all_passed=false
        fi
    fi

    # --- Step 6: 性能概览 ---
    log_step "性能概览..."

    # 页面大小估算
    local page_size=$(curl -s "$base_url/" --connect-timeout 5 | wc -c)
    log_info "📊 主页大小: $((page_size / 1024)) KB"

    # 加载时间测试
    local load_time=$(curl -s -o /dev/null -w "%{time_total}" "$base_url/" --connect-timeout 5)
    log_info "⏱️  加载时间: ${load_time}s"

    # --- Step 7: 结果汇总 ---
    echo ""
    echo "╔══════════════════════════════════════════════╗"
    echo "║              测试结果汇总                    ║"
    echo "╠══════════════════════════════════════════════╣"
    printf "║  %-20s %s\n" "服务地址:" "http://localhost:$PORT"
    printf "║  %-20s %s\n" "产物大小:" "$total_size"
    printf "║  %-20s %s\n" "文件数量:" "$total_files"
    printf "║  %-20s %s\n" "加载时间:" "${load_time}s"
    echo "╠══════════════════════════════════════════════╣"

    if [ "$all_passed" = true ]; then
        printf "║  ${GREEN}%-20s${NC} %s\n" "状态:" "✅ 所有检查通过"
    else
        printf "║  ${RED}%-20s${NC} %s\n" "状态:" "❌ 存在问题"
    fi
    echo "╚══════════════════════════════════════════════╝"
    echo ""

    # --- Step 8: 提示信息 ---
    log_info "💡 提示:"
    echo "   • 在浏览器中打开: http://localhost:$PORT"
    echo "   • 按 Ctrl+C 停止服务器"
    echo "   • 服务器日志: $LOG_FILE"
    echo ""

    # 保持服务器运行直到用户中断
    log_info "等待请求... (按 Ctrl+C 退出)"
    wait "$server_pid"
}

# --- 执行主函数 ---
main "$@"
