# NiuPanel

NiuPanel 是一个使用 Rust 与 Vue 3 构建的服务器运维、脚本执行和定时任务管理面板，提供任务调度、运行环境、文件与 Git 管理、分享分发、Web Terminal、插件应用、MCP 自动化以及系统更新回退能力。

项目使用 [Apache License 2.0](https://github.com/wyourname/NiuPanel/blob/dev/LICENSE) 开源，允许个人和商业使用、修改与分发，并提供明确的专利授权。

> [!WARNING]
> 项目仍处于 `1.0.0` 之前的快速迭代阶段，配置、接口和数据结构可能发生不兼容变化。生产部署前请备份完整的 `data/` 目录，并在升级前阅读发布说明。

> [!IMPORTANT]
> NiuPanel 可以执行脚本、打开终端、修改文件和安装软件包。请仅部署在可信环境中，为每个实例设置独立随机的 `SESSION_KEY`，并限制公网端口、MCP、API Key、插件和终端权限。

## 分支说明

- `main`：项目介绍与稳定发布入口。
- `dev`：当前开发源码和最新功能。

获取开发源码：

```bash
git clone -b dev https://github.com/wyourname/NiuPanel.git
cd NiuPanel
```

## 功能概览

| 模块 | 主要能力 |
| --- | --- |
| 概览 | 系统资源、任务统计、运行状态、下一次调度和实时活动事件 |
| 任务 | Python、Node.js、Shell，Cron/随机调度，批量操作，任务链和实时日志 |
| 变量 | 全局变量、任务变量、多任务关联、排序、导入和批量启停 |
| 环境 | Python 虚拟环境、Node.js 版本、系统软件包、依赖和镜像源管理 |
| 文件 | 在线编辑、上传下载、复制重命名、压缩包解压和 URL 下载 |
| Git | 仓库配置、同步、文件浏览、任务扫描和批量导入 |
| 分享 | `.npack` 打包、导入预览、来源跟踪、分享市场和中转站 |
| 扩展 | 插件安装、上传、市场、签名、权限预览、健康检查、更新和回滚 |
| 自动化 | OpenAPI、API Key、Webhook、MCP Server 和 Telegram |
| 系统 | 权限审计、会话安全、备份恢复、日志清理、Core/Web 独立更新回退 |

## 完整功能

### 概览与运行监控

- 展示 CPU、内存、运行任务、失败任务和下一次任务调度。
- 通过实时事件流更新任务和系统活动状态。
- 提供任务状态、运行耗时、PID、CPU 和内存使用信息。
- 桌面端采用多窗口工作区，移动端提供独立页面、抽屉和底部操作布局。

### 任务调度与执行

- 原生执行 Python、Node.js 和 Shell 脚本。
- Python 任务可选择独立 Virtualenv，Node.js 任务可选择已安装版本。
- 支持五段或六段 Cron 表达式、秒级调度和时区配置。
- 支持在指定时间范围内生成每日随机执行计划。
- 支持手动运行、停止、暂停、恢复、启用和停用。
- 支持任务置顶、搜索、分页和状态筛选。
- 支持批量运行、停止、暂停、恢复、启停、置顶和删除。
- 支持任务完成后触发后续任务，组合简单任务链。
- 支持从 Raw URL 快速下载脚本并创建任务。
- 可为任务声明 Python/Node.js 依赖，执行前同步运行环境。
- 可设置 CPU、内存和超时限制。
- 支持任务级变量和运行通知开关。
- 记录每次运行历史，可查看、搜索、流式跟踪或删除指定运行日志。
- 通过 SSE 推送实时日志和任务状态变化。

### 变量管理

- 支持全局变量和脚本任务作用域变量。
- 一个变量可以关联多个任务。
- 支持按名称、作用域和任务筛选。
- 支持单个或批量启用、停用和删除。
- 支持任务变量排序和变量批量导入。
- 任务执行时自动组合全局变量与当前任务变量。

### Python、Node.js 与系统环境

- 创建和删除 Python 虚拟环境。
- 安装、卸载并查看 Python 软件包。
- 安装、删除和切换 Node.js 版本，可设置默认版本。
- 管理不同 Node.js 环境的软件包。
- 查看、安装和卸载系统级软件包。
- 配置 Python、Node.js 和系统软件源镜像。
- 安装和卸载操作以后台作业运行，可查看实时日志、最终状态并取消作业。

### 文件管理器

- 浏览面板脚本目录和子目录。
- 在线读取、编辑和保存文本代码。
- 集成 Monaco Editor，支持常见脚本语言高亮。
- 创建文件和目录，支持复制、重命名和批量删除。
- 上传、下载单个文件或批量下载。
- 从 URL 下载文件到服务器。
- 安全解压 ZIP、TAR、TAR.GZ 和 TGZ 等归档。
- 路径规范化和归档解压会阻止目录穿越、符号链接和硬链接攻击。
- 支持常见图片文件预览。

### Git 仓库同步

- 配置和管理多个 Git 仓库。
- 支持仓库凭据、分支、代理和同步目录配置。
- 手动同步远程仓库内容。
- 在线浏览仓库文件树。
- 扫描仓库中的脚本和任务配置。
- 从扫描结果选择并导入任务。
- Git 操作通过独立权限控制，敏感凭据不会通过普通查询接口返回。

### 分享、导入与脚本市场

- 将任务、依赖文件和变量快照打包为 `.npack`。
- 支持密码、过期时间和一次性读取等分享限制。
- 分享导入采用提交、下载、状态查询、预览和确认的分阶段流程。
- 可选择需要导入的任务，并决定是否更新已有任务。
- 记录导入来源，支持查看来源分组和清理来源任务。
- 支持分享中转站容量、文件和内容更新管理。
- 支持配置多个脚本市场源、同步市场索引和聚合浏览脚本。
- 支持将分享包上传到中转站并查询传输状态。

### Web Terminal

- 基于 WebSocket 与 PTY 提供浏览器终端。
- 支持桌面工作区和移动端终端页面。
- 终端连接执行 Origin、Host 和端口校验。
- 终端属于高权限能力，建议只对可信管理员开放。

### 插件与扩展中心

- 支持进程插件、原生 Vue 插件应用和声明式主题插件。
- 支持本地目录安装、归档上传安装和插件市场安装。
- 安装或更新前展示权限、API allowlist、路由和版本影响预览。
- 支持启用、停用、卸载、版本历史和一键回滚。
- 提供插件健康检查、依赖检查和路由冲突检测。
- 支持 SHA-256 完整性校验和 Ed25519 签名验证。
- 进程插件在 Linux 上使用独立身份、清理后的环境、Landlock 和 seccomp 限制运行。
- 插件默认无网络访问；需要外部网络时必须声明 `network_outbound`。
- 插件 UI 通过 `@niupanel/plugin-sdk` 接入宿主导航、主题、通知和受控 API。
- 编译器、Agents 和计划中的 Telegram Bot 均可通过插件扩展。

### Python 脚本编译

- 通过编译器插件查询支持的 Python 版本。
- 将脚本提交给启用的编译器插件处理。
- Core 不内置私有编译实现，编译能力可独立安装和升级。

### MCP Server

- Core 在 `/mcp` 提供 Streamable HTTP MCP Server。
- Claude、Codex 等 MCP Client 可使用 `X-API-Key` 连接面板。
- MCP 工具覆盖系统状态、任务、日志、变量、环境、文件、后台作业、Git、分享和 Webhook。
- 每个工具直接复用任务、文件、环境等现有业务权限，不额外维护 MCP 专属权限。
- 工具调用写入系统审计日志。
- Host allowlist 用于降低 DNS rebinding 风险。
- NiuPanel 是 MCP Server，不负责安装或连接外部 MCP Server。

### API、Webhook 与开发者能力

- 创建、修改和撤销具有独立权限集合的 API Key。
- 提供 `/api/v1/documents/openapi.json` OpenAPI 文档。
- Web 设置页面内置 API 权限说明和文档入口。
- Webhook Push API 可通过面板通知渠道发送消息。
- 通知配置支持 Webhook 和 SMTP 邮件测试。

### Telegram

- 当前内置实现支持 Bot Token、代理、自定义 API 地址和连接测试。
- 支持任务管理、日志查询、消息与服务器文件发送。
- 支持自定义命令和事件工作流。
- 支持从 Telegram 接收脚本文件和分享包。
- 支持 Telegram 登录二次验证。
- Telegram 正在规划迁移为可独立安装的插件，迁移期间保留现有功能。

### 用户、安全与审计

- 首次启动通过引导创建管理员账号。
- 支持登录、退出、忘记密码、验证码重置和 Telegram 二次验证。
- 支持个人资料、密码、邮箱验证和界面偏好设置。
- 支持查看和撤销活动会话。
- 使用细粒度权限控制任务、变量、文件、Git、环境、插件、API Key、审计和 MCP 操作。
- 关键操作写入审计日志。
- Session Cookie 根据访问 Host 隔离，便于同一浏览器访问多个实例。
- CORS、MCP Host、插件签名和会话密钥均可独立配置。

### 备份、维护与版本回退

- 创建、下载、上传、恢复和删除系统备份。
- 可选择备份数据库、配置、Telegram 数据等内容。
- 支持运行日志清理和维护任务状态查询。
- Launcher 负责 Core 进程监督、候选版本健康检查和失败自动回退。
- Core 更新前保存 SQLite 主文件、WAL 和 SHM 快照。
- Core 与 Web UI 分别安装、激活、检查更新和回滚。
- 正式 GitHub Release 使用 Core 版本作为应用版本，`niupanel-release.json` 绑定 Core 与 Web 的完整校验信息；Docker 环境镜像独立发布，并以 OCI label 记录所含 Core 版本。
- Web 发布包使用逐文件 SHA-256 manifest 校验并原子切换。
- 内置 `/recovery` 恢复入口不依赖当前 Web UI 包。

## 快速开始

### Docker

必须持久化完整的 `/app/data`，不要只挂载 SQLite 文件。

```bash
docker run -d \
  --name niupanel \
  --restart unless-stopped \
  -p 7788:7788 \
  -v "$(pwd)/data:/app/data" \
  wyourname/niupanel:latest
```

镜像使用多架构 Manifest，Docker 会自动选择 AMD64、ARM64 或 ARMv7 产物。正式标签直接使用 Docker 环境版本，例如 `wyourname/niupanel:3.0.0`；`latest` 是滚动别名。需要指定架构时使用 `3.0.0-amd64`、`3.0.0-arm64` 或 `3.0.0-armv7`。Docker 镜像通过单独的 Actions 工作流构建，并在 OCI label 中记录所含 Core 版本和构建源码提交。

启动后访问：

```text
http://<服务器地址>:7788
```

首次访问会进入初始化引导。NiuPanel 使用嵌入式 SQLite，不需要额外部署数据库服务。

### 环境变量

至少建议配置：

```env
SERVER_ADDR=0.0.0.0:7788
DATABASE_URL=sqlite://data/database/niupanel.db?mode=rwc
SESSION_KEY=<每个实例独立的随机值>
# 远程插件市场包的签名校验；管理员直接上传不需要手工填写签名
PLUGIN_SIGNATURE_REQUIRED=true
MCP_ALLOWED_HOSTS=localhost,127.0.0.1,::1
TRUSTED_PROXIES=127.0.0.1,::1
SESSION_COOKIE_SECURE=true
```

可以使用以下命令生成 Session Key：

```bash
openssl rand -base64 64
```

完整示例见 [`dev` 分支的 `.env.example`](https://github.com/wyourname/NiuPanel/blob/dev/.env.example)。

## 开发环境

项目提供 Docker 开发环境：

```bash
./start.sh
./start.sh logs
./start.sh config
./start.sh down
```

默认地址：

| 服务 | 地址 |
| --- | --- |
| Vue/Vite | `http://127.0.0.1:7787` |
| Rust API | `http://127.0.0.1:7788` |

开发容器会将 Cargo、pnpm、构建产物和运行数据放入 Docker Volume，源码通过 bind mount 实时同步。

常用检查：

```bash
cargo fmt --all -- --check
cargo test --workspace

cd niupanelweb
pnpm exec vue-tsc --noEmit
pnpm run build
pnpm run verify:ui-design-system
```

## 技术架构

| 层级 | 技术 |
| --- | --- |
| Core | Rust、Tokio、Axum、SeaORM、SQLite |
| 调度与执行 | tokio-cron-scheduler、PTY、SSE、WebSocket |
| Launcher | Rust、健康检查、版本激活、SQLite 快照回退 |
| Web | Vue 3、TypeScript、Vite、Element Plus、UnoCSS |
| 插件 | Rust Plugin Host、JSON Lines Process Protocol、Vue Plugin SDK |
| 自动化 | OpenAPI、MCP Streamable HTTP、API Key |

主要目录：

```text
niupanel/           HTTP API 与应用组合
niupanel-core/      调度、执行、环境和系统核心能力
niupanel-launcher/  Core 激活、健康检查与回退
niupanel-plugin/    插件 manifest、沙箱、安装和运行时
niupanel-bot/       迁移中的内置 Telegram 实现
niupanelweb/        Vue Web UI
packages/           TypeScript 公共包和插件 SDK
examples/           可运行插件示例与模板
docs/               架构、集成和开发文档
```

## 文档

- [系统架构](https://github.com/wyourname/NiuPanel/blob/dev/docs/architecture/system-architecture.md)
- [仓库目录与模块边界](https://github.com/wyourname/NiuPanel/blob/dev/docs/architecture/repository-layout.md)
- [前端设计系统](https://github.com/wyourname/NiuPanel/blob/dev/docs/frontend/design-system.md)
- [插件开发](https://github.com/wyourname/NiuPanel/blob/dev/docs/plugins/plugin-development.md)
- [Telegram Bot 插件化计划](https://github.com/wyourname/NiuPanel/blob/dev/docs/plugins/telegram-bot.md)
- [MCP 接入规范](https://github.com/wyourname/NiuPanel/blob/dev/docs/integrations/mcp.md)

## 参与贡献

欢迎提交 Issue 和 Pull Request。贡献前请阅读：

- [贡献指南](https://github.com/wyourname/NiuPanel/blob/dev/CONTRIBUTING.md)
- [安全策略](https://github.com/wyourname/NiuPanel/blob/dev/SECURITY.md)
- [行为准则](https://github.com/wyourname/NiuPanel/blob/dev/CODE_OF_CONDUCT.md)

安全漏洞请通过 GitHub Security Advisory 私下报告，不要直接创建公开 Issue。

## 许可证

NiuPanel 使用 [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0)。除非文件另有声明，提交到本仓库的贡献均按同一许可证提供。

第三方依赖和发布包中包含的工具仍受各自上游许可证约束。

## 免责声明

- 请勿将本软件用于违反适用法律法规的用途。
- 在生产环境启用终端、脚本执行、文件管理、MCP 或第三方插件前，请完成独立安全评估。
- 请自行维护数据库、脚本、密钥和版本升级前备份。
- 软件按 Apache-2.0 的“按现状”条款提供，不附带任何明示或默示担保。
