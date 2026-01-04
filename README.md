# NiuPanel

基于 Rust 开发的高性能服务器运维与定时任务管理面板。
主打轻量、稳定、环境隔离。非开源项目。

## 功能特性

### 任务调度与执行
*   **多语言支持**: 原生支持 Python (Virtualenv 隔离)、Node.js、Shell 脚本。
*   **调度模式**: 支持标准 Crontab 表达式，精确到秒级。支持手动触发、停止、暂停、恢复。
*   **实时日志**: 基于 SSE (Server-Sent Events) 的实时日志流推送，支持历史日志回溯。
*   **资源限制**: 可配置单任务 CPU 和 内存 限制，防止脚本资源泄露。

### 环境与依赖管理
*   **Python**: 提供 Web 图形化界面管理 Virtualenv，支持一键创建环境、pip 包安装/卸载、镜像源配置。
*   **Node.js**: 全局 npm 包管理。
*   **系统级**: 支持 apt/pkg 系统包管理。

### 运维工具
*   **Web Terminal**: 集成 WebSocket + PTY 实现的 Web 终端。
*   **文件管理**: 全功能在线文件管理器，支持编辑、上传、下载、权限修改。
*   **Git 同步**: 支持绑定 Git 仓库，自动化同步脚本文件，支持从仓库扫描并导入任务。
*   **脚本编译**: 提供 Python 脚本加密/编译功能，保护核心代码。

### 高效分发与流转
*   **URL 极速创建**: 支持直接通过 Raw URL (GitHub/Gist等) 拉取脚本。系统自动分析文件头（Shebang）识别语言环境（Python/Node/Shell），自动赋予执行权限并生成随机 Cron 调度，实现“从链接到任务”的秒级部署。
*   **闭环分享体系**:
    *   **一键打包**: 将多任务及其依赖文件、环境变量快照打包为 `.npack` 专有格式。
    *   **动态更新**: 分享源支持“重打包”更新，已导入该分享的终端用户可执行“导入更新”，无缝同步上游代码变更，解决脚本分发后的版本碎片化问题。
    *   **安全交付**: 支持配置提取密码、有效期限制及阅后即焚（一次性链接），确保交付安全。

### 系统安全
*   **权限控制**: 细粒度 RBAC (Role-Based Access Control) 权限模型。
*   **安全审计**: 记录核心操作日志（登录、任务增删改、文件操作等）。
*   **API Access**: 支持生成系统级 API Key，便于第三方集成。
*   **分享中心**: 支持脚本/任务打包分享，配置阅后即焚、密码保护及有效期。

## 部署说明

推荐使用 Docker 容器化部署，确保环境一致性。

### Docker 启动

x86_64:
```bash
docker run -d \
  --name niupanel \
  --restart unless-stopped \
  -p 7788:7788 \
  -v $(pwd)/data:/app/data \
  wyourname/niupanel:amd64-latest
```
ARM64:
```bash
docker run -d \
  --name niupanel \
  --restart unless-stopped \
  -p 7788:7788 \
  -v $(pwd)/data:/app/data \
  wyourname/niupanel:arm64-latest
```
ARMv7:
```bash
docker run -d \
  --name niupanel \
  --restart unless-stopped \
  -p 7788:7788 \
  -v $(pwd)/data:/app/data \
  wyourname/niupanel:armv7-latest
```


### 目录映射

| 容器路径 | 说明 | 宿主机建议 |
| :--- | :--- | :--- |
| `/app/data` | **核心数据目录**。包含数据库(SQLite)、脚本文件、日志、密钥等。 | **必须持久化** |

### 初始化

1.  服务启动后，访问 `http://<IP>:7788`。
2.  首次访问将引导创建管理员账号（Admin）。
3.  系统基于 SQLite，无需额外部署数据库服务。

## 技术规格

*   **后端**: Rust (Tokio, Axum, SeaORM)
*   **前端**: Vue 3 (Vite)
*   **数据库**: SQLite (Embedded)
*   **内存占用**: 闲置 < 50MB (视具体任务量而定)

## 免责声明

*   本软件为闭源分发，禁止反编译、破解或用于商业用途（除非获得授权）。
*   请勿将本软件用于任何违反当地法律法规的用途。
*   使用本软件产生的任何数据丢失或服务器故障，开发者不承担连带责任。
