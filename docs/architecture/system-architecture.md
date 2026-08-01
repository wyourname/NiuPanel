# NiuPanel 系统架构

## 产品边界

| 模块 | 归属 | 职责 |
| --- | --- | --- |
| launcher | 公开 Core | Panel 进程监督、发布事务、健康检查、SQLite 快照和失败恢复 |
| Core | 公开 Core | API、任务、权限、插件宿主、面板 MCP Server |
| Web UI | 独立构建组件 | Vue 3 面板，由 Panel Release 与匹配的 Core 一起激活或回退 |
| 插件平台 | Core 扩展能力 | 应用安装、签名、版本、原生 Vue UI 和进程协议 |
| 业务扩展 | 独立插件 | Agents、编译器等可选能力通过插件交付 |
| Telegram Bot | 迁移中的独立插件 | 当前内置实现保持兼容，目标由插件负责长驻连接、通知和远程操作 |
| MCP | Core 系统能力 | 通过 `/mcp` 让外部 MCP Client 操控当前面板 |

MCP 不属于扩展中心，也不负责连接外部 MCP Server。扩展中心只管理可安装插件。

## 版本契约

Core 与 Web 是不可变组件，Panel Release 是用户可见、可激活的唯一发布单元。它们共享以下兼容字段：

- `launcher_protocol`：launcher 与 Core 激活事务协议。
- `api_contract`：Web、Open API 和 Core 的接口契约代次。
- `schema_epoch`：数据库向后兼容边界。
- `schema_revision`：当前 epoch 内的迁移修订号。

同一 `schema_epoch` 内只允许 expand-first 迁移：新增表、可空字段、兼容索引和双读双写。删除字段、重命名字段、改变含义等破坏性变更必须延后到新的 epoch。生产回退不执行 `migration down`。

Core 使用 `Core-vX.Y.Z` Tag，Web 使用 `web-vX.Y.Z` Tag，组件 Tag 永远不覆盖。Panel 使用 `vX.Y.Z` 或 `vX.Y.Z-beta.N` Tag；GitHub prerelease 状态与 `preview` 通道一致。`main/release/channels/preview.json` 与 `stable.json` 是唯一通道指针，每个文件只指向一个完整 Panel Release，并包含经验证的 Core/Web 组件描述。切换通道只改变检查来源，不自动安装。0.8.0 是新协议的最低基线，不读取 0.7.x 状态格式。

## Panel 更新与回退

Core 包含 `niupanel`、`niupanel-launcher`、构建期 manifest 与按架构准备的运行时工具；Web 包包含 Vite 构建结果与构建期 manifest。manifest 只用于发布和安装校验，不复制到运行目录。运行目录：

```text
data/system/
  runtime.db
  releases/panel/<panel-version>/
    core/
    web/
  web/current -> ../releases/panel/<panel-version>/web
  snapshots/<transaction>/
```

激活流程：

1. Core 校验通道、组件契约、归档 SHA-256、内部 manifest 和文件完整性，只下载发生变化的组件。
2. Core 在同一文件系统的 staging 目录组装完整 Panel Release，原子移动到不可变版本目录，再将 pending activation 写入 `runtime.db` 后退出。
3. Launcher 在旧进程完全退出后创建并持久化不可覆盖的数据库快照；中断重试前先恢复该快照。
4. Launcher 启动候选 Panel，检查包含 Panel 版本的 `/healthz`，并执行稳定观察窗口。
5. 成功时在同一个 SQLite 事务中提交 active/previous 与激活日志；失败时恢复快照，并原子记录失败事务。

回退始终恢复目标版本离开时的数据库快照，并要求用户显式确认该快照之后的数据会丢失；不执行 `migration down`，也不允许绕过 Launcher 直接切换文件。

## Web 组件发布

Web 可以独立构建和发布，因此纯前端修复不需要重建 Core。它不能独立激活：维护者使用现有 Core Tag 与新 Web Tag 组合一个新的 Panel Release，Launcher 随后把完整发布作为一个事务切换。manifest 包含 Web 版本、API contract、Core 最低/最高版本和逐文件 SHA-256；安装器拒绝路径穿越、链接文件、缺失文件和哈希不一致。

`/recovery` 是编译进 Core 的最小恢复控制台，不依赖当前 Vue 包。正常面板无法加载时，管理员仍可登录并回退完整 Panel Release。

## MCP Server

NiuPanel 在 `/mcp` 提供 Streamable HTTP MCP Server，统一使用 `X-API-Key`：

- API Key 完成身份认证，每个工具直接复用任务、文件、环境等现有业务权限。
- “MCP 工具预设”只是现有权限组合，不引入 MCP 专属权限体系。
- 只暴露明确注册的面板工具，不从插件或外部 MCP 动态导入工具。
- 工具调用写入统一审计日志。
- MCP Host 校验默认关闭，可通过 `MCP_ALLOWED_HOSTS` 显式启用 allowlist；公网部署必须使用 HTTPS。

## 发布验证

- Core、Launcher、Web 和 Panel 通道分别通过构建、契约与状态机测试。
- Core 与 Web 各自发布不可变归档；Panel 发布只组合已经存在的组件，不重复打包二进制。
- 内置更新只读取当前通道指向的 Panel Release，精确校验版本、架构、文件名、大小和 SHA-256 后才安装。
- Docker 环境版本独立维护为 `3.0.1`。只有容器基线需要更新时才手动构建，推送 `3.0.1` 与 `latest`，不使用 preview 镜像标签，也不在界面展示组件组合。
- Docker 由 launcher 作为 PID 1 启动，并持久化整个 `/app/data`。
- 容器启动时，Launcher 会比较镜像内置 Panel 与 `runtime.db` 的 active 版本：仅当内置版本更高时才通过同一 pending/快照/健康检查事务激活；版本相同保持不可变，版本更低绝不降级。
- 发布检查应覆盖候选启动失败、Launcher 中断重试、数据库恢复、Panel 回退和 MCP 鉴权。
- 清理策略不得删除 active、previous、pending 或回退链依赖的发布目录和快照。
