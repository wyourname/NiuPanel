# NiuPanel MCP Server 接入规范

NiuPanel 自身是 MCP Server。Claude、Codex 或其他支持 Streamable HTTP 的 MCP Client 通过 MCP 工具操控当前面板；NiuPanel 不负责安装或连接外部 MCP Server，MCP 也不属于插件系统。

## 连接信息

| 项目 | 值 |
| --- | --- |
| 传输协议 | Streamable HTTP |
| MCP Endpoint | `https://panel.example.com/mcp` |
| 鉴权 Header | `X-API-Key: <key>` |
| 必需权限 | 所调用工具对应的现有业务权限 |
| 管理信息 | `GET /api/v1/mcp/info` |

`/api/v1/mcp/info` 供已登录的 Web UI 展示连接地址和工具权限。真正的 MCP JSON-RPC、SSE 和会话请求只经过 `/mcp`。

## 创建 API Key

在“设置 -> API 访问”创建独立密钥，并选择工具对应的业务权限，例如 `task:list`、`task:read`、`task:run`。“MCP 工具预设”只是这些现有权限的快捷组合，不会创建另一套 MCP 权限。

不要给自动化客户端授予 `*:*`。读取、执行和停止任务应使用不同权限组合；每次 MCP 工具调用都会写入审计日志。

## 客户端配置

不同 MCP Client 的配置字段略有差异，核心参数如下：

```json
{
  "mcpServers": {
    "niupanel": {
      "type": "streamable-http",
      "url": "https://panel.example.com/mcp",
      "headers": {
        "X-API-Key": "npk_xxx"
      }
    }
  }
}
```

API Key 只通过 `X-API-Key` 发送，不使用 `Authorization: Bearer`。

## 当前工具

| 工具 | 权限 | 说明 |
| --- | --- | --- |
| `system_status` | `overview:read` | 读取 Core、Web、Schema 和任务摘要 |
| `tasks_list` | `task:list` | 按关键字和状态查询任务列表 |
| `tasks_get` | `task:read` | 查询任务详情 |
| `tasks_get_logs` | `task:read` | 读取最新任务日志尾部 |
| `tasks_history` | `task:read` | 查询任务历史运行记录 |
| `tasks_get_run_log` | `task:read` | 读取指定历史运行日志 |
| `tasks_create` | `task:create` | 创建任务 |
| `tasks_update` | `task:update` | 更新任务配置 |
| `tasks_delete` | `task:delete` | 删除任务，可选择同时删除脚本和任务变量 |
| `tasks_run` | `task:run` | 启动任务 |
| `tasks_stop` | `task:stop` | 停止任务 |
| `tasks_pause` | `task:stop` | 暂停任务 |
| `tasks_resume` | `task:stop` | 恢复任务 |
| `tasks_enable` | `task:update` | 启用任务调度 |
| `tasks_disable` | `task:update` | 停用任务调度 |
| `environments_list` | `env:list` | 查询可用运行环境 |
| `environments_get` | `env:read` | 按名称读取运行环境详情 |
| `environments_versions` | `env:read` | 查询可安装的 Python 和 Node.js 版本 |
| `environments_packages_list` | `env:read` | 查询 Python、Node.js 或系统软件包 |
| `environments_create` | `env:create` | 创建 Python 环境或安装 Node.js 版本，返回后台作业 ID |
| `environments_install_packages` | `env:update` | 安装 Python、Node.js 或系统软件包，返回后台作业 ID |
| `environments_uninstall_package` | `env:delete` | 卸载 Python、Node.js 或系统软件包 |
| `environments_delete` | `env:delete` | 删除 Python 环境或卸载 Node.js 版本 |
| `environments_set_default` | `env:update` | 设置系统默认 Node.js 版本 |
| `variables_list` | `var:list` | 查询变量元数据，不返回变量值 |
| `variables_get` | `var:read` | 读取指定变量及其敏感值 |
| `variables_create` | `var:create` | 创建变量 |
| `variables_update` | `var:update` | 更新变量 |
| `variables_delete` | `var:delete` | 删除变量 |
| `audit_list` | `audit:list` | 查询最近的审计记录 |
| `files_list` | `file:list` | 列出脚本目录内容 |
| `files_read` | `file:read` | 读取不超过 2 MiB 的 UTF-8 脚本文件 |
| `files_write` | `file:write` | 写入不超过 2 MiB 的脚本文件 |
| `files_create_directory` | `file:write` | 创建脚本目录 |
| `files_delete` | `file:delete` | 删除脚本文件或目录 |
| `jobs_list` | `job:list` | 查询系统后台作业 |
| `jobs_get` | `job:read` | 读取后台作业详情 |
| `jobs_get_logs` | `job:read` | 读取后台作业日志 |
| `jobs_cancel` | `job:*` | 取消正在运行的后台作业 |
| `webhook_push` | `webhook:push` | 通过面板通知渠道推送消息 |
| `share_station_stats` | `share:list` | 读取分享中转站容量和配置状态 |
| `share_station_files` | `share:list` | 列出中转站文件，不返回分享密码 |
| `share_import_sources` | `share:list` | 读取已导入任务的来源摘要 |
| `git_repos_list` | `git:read` | 列出仓库和同步状态，不返回访问令牌或代理地址 |
| `git_repo_files` | `git:read` | 浏览指定仓库文件 |
| `git_repo_scan_tasks` | `git:read` | 扫描可导入任务，不执行导入 |
| `git_repo_sync` | `git:sync` | 同步指定仓库 |
| `system_releases` | `overview:read` | 读取已安装 Panel 版本和回退状态 |
| `system_update_check` | `overview:read` | 按当前 stable/preview 通道检查 Panel 更新 |

后续工具继续按面板业务域增加，不通过插件动态注入。插件可以拥有自己的 API 和 UI，但不能替代系统 MCP 权限边界。

`variables_list` 有意不返回变量值。只有显式授予 `var:read` 的 API Key 才能通过 `variables_get` 读取敏感值。任务和变量删除工具属于破坏性操作，应为自动化客户端单独授权。

文件工具只能访问面板配置的脚本目录，并复用 Web 文件管理器的路径规范化与目录穿越检查。`files_write`、`files_delete`、`jobs_cancel` 等工具应仅授予受控自动化客户端。

环境创建、Python/Node 依赖安装和依赖卸载会返回后台作业 ID，可继续调用 `jobs_get` 和 `jobs_get_logs` 查询进度。系统包安装会直接修改宿主运行环境；环境删除和包卸载属于破坏性操作，应仅向受控客户端授予 `env:delete`。

Git 工具不会返回仓库访问令牌或代理地址。Share 工具不会返回文件密码。更新检查最多返回截断后的发布摘要，不提供安装或版本激活能力。

## Host 校验

MCP Streamable HTTP 默认不校验 Host，可直接通过本机地址、域名、反向代理或端口映射访问。需要降低 DNS rebinding 风险时，可显式启用 Host allowlist：

```bash
MCP_ALLOWED_HOSTS=panel.example.com,localhost,127.0.0.1
```

未设置或删除 `MCP_ALLOWED_HOSTS` 时保持默认开放。allowlist 中只填写 Host 或 `Host:port`，不要包含协议和路径。

## 运维边界

- MCP Server 与 Core 同进程启动，不需要插件安装或热加载。
- API Key 被撤销或权限修改后，后续请求立即受统一鉴权控制。
- MCP 不保存外部 MCP 服务连接配置。
- 外部客户端应使用 HTTPS 反向代理访问，不要直接暴露未加密的公网端口。
