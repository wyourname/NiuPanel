# NiuPanel 系统架构

## 产品边界

| 模块 | 归属 | 职责 |
| --- | --- | --- |
| launcher | 公开 Core | Core 进程监督、健康检查、SQLite 快照、自动回退 |
| Core | 公开 Core | API、任务、权限、插件宿主、面板 MCP Server |
| Web UI | 独立发布 | Vue 3 面板，可独立安装、激活和回退 |
| 插件平台 | Core 扩展能力 | 应用安装、签名、版本、原生 Vue UI 和进程协议 |
| 业务扩展 | 独立插件 | Agents、编译器等可选能力通过插件交付 |
| Telegram Bot | 迁移中的独立插件 | 当前内置实现保持兼容，目标由插件负责长驻连接、通知和远程操作 |
| MCP | Core 系统能力 | 通过 `/mcp` 让外部 MCP Client 操控当前面板 |

MCP 不属于扩展中心，也不负责连接外部 MCP Server。扩展中心只管理可安装插件。

## 版本契约

Core 与 Web 分别发布版本，并共享以下兼容字段：

- `launcher_protocol`：launcher 与 Core 激活事务协议。
- `api_contract`：Web、Open API 和 Core 的接口契约代次。
- `schema_epoch`：数据库向后兼容边界。
- `schema_revision`：当前 epoch 内的迁移修订号。

同一 `schema_epoch` 内只允许 expand-first 迁移：新增表、可空字段、兼容索引和双读双写。删除字段、重命名字段、改变含义等破坏性变更必须延后到新的 epoch。生产回退不执行 `migration down`。

正式应用发布使用 Core 版本作为 Git Tag，且统一采用 `vX.Y.Z` 纯数字形式。新 Tag 只构建一次并先创建 GitHub Pre-release；测试通过后将同一个 Release 提升为正式版，不重建或替换资产。Web、Launcher 与 Docker 分别保留独立组件版本。应用 CI 生成 schema 2 的 `niupanel-release.json`，记录 Git SHA、Core 三架构包、Web 包、兼容契约、大小及全部 SHA-256；该文件不保存通道，preview/stable 以 GitHub Release 状态为准。Docker 由独立工作流从已经提升为正式版并经该 manifest 校验的资产构建；Docker 正式标签使用环境版本，并通过 OCI label 记录所含 Core/Web 版本。Core/Launcher 通过 `launcher_protocol` 和 `RELEASE_PROTOCOL_VERSION` 验证兼容性，Web 通过 `api_contract` 和 `core.min/max` 加入同一发布。

## Core 更新与回退

Core 发布包只包含 `niupanel`、`niupanel-launcher`、`core-release.json` 与按架构准备的运行时工具；它不包含 Web UI。首次手动部署必须从同一 `niupanel-release.json` 同时取得匹配的 Web 包，或直接使用已完成该组合的 Docker / Magisk 安装方式。运行目录：

```text
data/system/
  releases/core/<version>/
  core/state.json
  core/transactions/<transaction>.json
  snapshots/<transaction>/
```

激活流程：

1. Core 校验包结构、launcher 协议和二进制 SHA-256，安装到不可变版本目录。
2. Core 写入 pending activation 后退出。
3. launcher 在旧 Core 完全退出后复制 SQLite 主文件、WAL 和 SHM 快照。
4. launcher 启动候选 Core，检查 `/healthz` 并执行试运行窗口。
5. 成功后提交 active/previous；失败则恢复数据库快照并启动上一版。

同 epoch 手动回退保留当前数据库。跨 epoch 回退必须找到进入新 epoch 前的快照，并要求用户确认升级后数据会丢失。

## Web UI 独立发布

Web 发布包只包含 Vite 构建结果和 `release-manifest.json`：

```text
data/system/
  releases/web/<version>/
  web/state.json
  web/current -> ../../releases/web/<version>
```

manifest 包含 Web 版本、API contract、Core 最低/最高版本和逐文件 SHA-256。Core 在 staging 目录解包并拒绝路径穿越、链接文件、缺失文件和哈希不一致；通过后使用原子 symlink 切换，无需重启后端。

`/recovery` 是编译进 Core 的最小恢复控制台，不依赖当前 Vue 包。正常面板无法加载时，管理员仍可登录并切换 Core/Web 版本。

## MCP Server

NiuPanel 在 `/mcp` 提供 Streamable HTTP MCP Server，统一使用 `X-API-Key`：

- API Key 完成身份认证，每个工具直接复用任务、文件、环境等现有业务权限。
- “MCP 工具预设”只是现有权限组合，不引入 MCP 专属权限体系。
- 只暴露明确注册的面板工具，不从插件或外部 MCP 动态导入工具。
- 工具调用写入统一审计日志。
- MCP Host 校验默认关闭，可通过 `MCP_ALLOWED_HOSTS` 显式启用 allowlist；公网部署必须使用 HTTPS。

## 发布验证

- Core、launcher、Web 分别通过构建和契约测试。
- Release 同时发布三个按架构区分的 Core 包、一个与架构无关的 Web 包和 schema 2 的 `niupanel-release.json`。CI 拒绝 Core/Web 混装归档，避免 Web 单独升级时被旧 Core 包覆盖；本地 `build.sh` 也生成并验证同一契约。
- 内置 Core/Web 更新先选择 GitHub Release 状态对应的 stable/preview Release，再强制读取 `niupanel-release.json`，精确校验版本、架构、文件名、大小和 SHA-256 后才下载或安装组件。
- Docker 环境版本独立维护。维护者从已提升为稳定版的应用 Tag 手动触发 Docker 工作流；该工作流先按同一份 `niupanel-release.json` 校验三架构 Core 包和唯一的 Web 包，再推送环境版本标签和 `latest`。
- Docker 由 launcher 作为 PID 1 启动，并持久化整个 `/app/data`。
- 发布检查应覆盖候选启动失败自动回退、Web 安装/切换/回退和 MCP 鉴权。
- 最近至少保留 3 个 Web 版本；Core 版本和数据库快照按事务保留策略清理，不允许删除 active、previous 或回退链依赖的快照。
