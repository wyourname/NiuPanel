# Telegram Agent 通道

## 设计目标

Telegram 是 Ops Agent 的一个可信输入输出通道，不是另一套面板命令系统。用户直接描述目标，Agent 负责诊断、选择工具和解释结果；Core 只负责与模型能力无关的传输、安全和生命周期。

## 职责边界

| 组件 | 职责 |
| --- | --- |
| Core Transport | Long Polling、Bot Token、可信 Chat ID、代理、自定义 Telegram API 地址、通知、启动与停止 |
| Ops Agent 插件 | 自然语言、会话、记忆、诊断、工具选择、确认语义和 Agent 运行审计 |
| Agent Tool Gateway | manifest 工具白名单、真实用户权限、执行、确认单、幂等与 Core 审计 |

Core 不再实现 `/task`、`/var`、Shell 命令、callback、自定义 commands、workflows、脚本上传或分享包导入。Core 仅保留 `/start`、用于新建会话的 `/new`，以及立即停止当前会话运行的 `/cancel`；其他文本消息原样交给配置的 Agent 插件。

## 调用链

```text
Telegram update
  -> Core 匹配 Chat ID 与 Topic ID
  -> Core 校验 Telegram from.id 发送者白名单
  -> Core 判断群组 mention/direct 唤醒条件
  -> Core 限量下载并校验受支持的文本附件
  -> Core 将绑定映射到面板 user_id
  -> Core 重新加载该用户的角色与权限
  -> PluginActionCaller::Telegram
  -> Ops Agent chat action
  -> Agent Tool Gateway
  -> Panel MCP tool router
  -> 权限校验、业务服务与审计
```

每个发送者使用 `telegram:<user_id>:<chat_id>:<topic_id>:<telegram_user_id>` 会话 ID。Gateway 的确认作用域同时包含插件 ID、真实面板用户、Chat、Topic 和 Telegram 发送者，因此群组成员、其他 Chat、Web 会话或 API Key 不能读取会话或确认该操作。绑定用户被删除后，后续 Agent 调用会立即失败，不会回退为管理员身份。

私聊在发送者白名单为空时只接受 `from.id == chat_id` 的消息。群组必须显式配置 `telegram_user_ids`；匿名管理员、频道代发和不在白名单内的成员会被静默忽略。群组可选择 `mention` 模式，仅在提及 Bot、回复 Bot 或使用 `/start`、`/new` 时唤醒，也可为专用运维群选择 `direct` 模式。Topic 绑定优先于整个 Chat 的通用绑定，输入、typing 状态和响应都留在原 Topic。

未绑定或发送者未授权的会话执行 `/start` 时，Transport 只返回当前 Chat ID、Topic ID 和 Telegram User ID，便于管理员录入绑定；该发现流程不会创建 Agent 会话、注入工具或访问任何面板数据。

## 文本附件

Telegram 文档的 caption 作为用户问题；没有 caption 时默认请求 Agent 分析附件。群组 `mention` 模式仍要求 caption 提及 Bot 或该文档回复 Bot，未绑定 Chat、未授权发送者和未命中唤醒条件的消息不会触发文件下载。

- 单个源文件最大 512 KiB，只接受日志、JSON/YAML/TOML、Markdown、常见代码和配置等 UTF-8 文本；
- 拒绝二进制 NUL、无效 UTF-8、图片、音频、视频，以及 `.env`、私钥和明显的 credentials/secrets 文件；
- 下载使用流式硬上限，Telegram 元数据与实际下载长度都会校验；提取文本按 Unicode 字符边界截断，避免撑满模型上下文；
- 附件通过 `chat.attachments[]` 结构化传递，backend 再次校验并递归脱敏 JSON 敏感键、文本凭据赋值和私钥块；
- 模型提示将附件明确标记为不可信数据。附件正文不会进入通知回复上下文、会话历史或长期工具审计。

当前 backend 不是多模态模型链路，因此图片及其他媒体不会被静默忽略或伪装成已分析；Transport 会在原 Topic 明确返回不支持提示。

## 通知回复上下文

任务结果、系统告警和普通系统通知发送成功后，Transport 会短期保存 `(chat_id, message_id)` 对应的结构化上下文。用户回复该 Telegram 消息时，Core 将关联的任务 ID、作业 ID、状态和事件类型放入 `panel_context.notification_reply`，Agent 因而可以直接查询最新日志或继续处置，无需用户复制 ID。

回复通知并发送精确短语“诊断”“诊断一下”“分析原因”“检查原因”“排查原因”“diagnose”或 `/diagnose` 时，Ops Agent 进入 `notification_read_only_diagnostic` 模式。任务通知优先查询任务详情、历史和日志；系统作业优先查询作业详情和日志；没有结构化资源 ID 的系统告警先读取 `system_status`，并明确说明证据边界。该模式会在模型可见工具列表中移除全部写工具，只能返回结论、证据、影响和建议；用户需要处置时，必须在后续普通消息中另行提出，再进入标准确认流程。

- 上下文严格按 Chat 和消息 ID 隔离，默认 6 小时过期，全局最多保留 2048 条；
- 任务输出、日志和凭据不会写入回复上下文；
- 上下文只负责定位对象，通知中的旧状态不能替代工具的实时查询结果；
- `/new` 同时删除当前发送者在该 Topic 的 Agent 会话并取消其待确认操作，避免跨成员或跨 Topic 继承确认单。

## 操作确认

- 查询类工具直接执行。
- 修改类工具首次调用只返回 `confirmation_required`，确认单默认 10 分钟有效。
- Agent 必须说明将执行的操作，并等待用户明确确认。
- 明确确认后，Agent 使用 `niupanel_confirm_operation` 执行；拒绝时使用 `niupanel_cancel_operation`。
- 多个待确认操作必须先让用户选择，不能推断确认目标。
- 只有 Core 返回 `status=completed` 后，Agent 才能宣称操作完成。
- 重复确认返回已缓存结果，不会重复执行；确认、取消和失败均写入 Core Audit。

## 并发、取消与幂等

会话键由面板用户、Chat、Topic 和 Telegram 发送者共同组成。同一会话中的消息严格按接收顺序执行，避免后一条问题越过前一条修改上下文；不同会话可以并行，Transport 默认最多同时运行 2 个 Agent 请求。插件 worker 暂时繁忙时，可取消调用会等待容量，不会直接返回并发上限错误。

- `/cancel` 以及精确的“停止”“取消”“停止诊断”“取消诊断”“stop”“cancel”不进入会话队列，可立即取消当前会话中正在运行和等待的请求；“停止任务 42”等带目标的操作仍交给 Agent；
- `/new` 会先取消当前会话的运行和排队请求，再删除对应 Agent 会话及待确认操作；
- JSON Lines 插件请求取消或调用 Future 被丢弃时，Core 会终止并丢弃当前 worker，同时释放进程池和并发 permit，后续请求会使用干净的新 worker；
- 强制终止插件 worker 后，Core 会调用 `session_interrupt`，把该会话遗留的 `running` run 收敛为 `cancelled`；清理失败会记录日志，但不会把用户的取消结果改写成普通插件错误；
- 已经由管理员明确确认并进入执行阶段的面板写操作在独立任务中运行到终态。取消 Agent 只停止后续分析，不回滚或重复执行已经确认的操作；最终状态继续写入确认存储和审计。

Transport 在调用 Agent 前持久化消息幂等账本，键为 `bot_id:chat_id:message_id`，默认保留 24 小时且最多 2048 条。重复 update 不会再次调用 Agent；Core 重启后发现上一进程留下的 `processing` 条目时，将其标记为 `interrupted` 并要求用户重新发送，避免对结果未知的消息自动重放。账本位于 `Config.system_dir/telegram-agent-message-ledger.json`，使用原子替换写入，Unix 下权限为 `0600`。

处理期间 Transport 先发送一条进度消息，并在读取附件、等待同会话前序消息、等待运行槽位、Agent 分析、调用具体工具和整理结果时编辑同一条消息。最终答案替换该进度消息，超出 Telegram 长度限制的后续内容才另行分块发送，且始终保留在原 Topic。

## 可用性与安全

Transport 保留在 Core，确保模型端点或 Agent 插件不可用时，系统通知和通道生命周期仍可工作。Bot Token 不进入模型上下文或配置查询响应；Agent 只能获得 Core 根据 manifest 和绑定用户真实权限注入的工具定义，也不能直接访问面板数据库。

每个 Chat/Topic 绑定独立配置 `success`、`failed`、`alert` 和 `notification` 通知事件。普通任务结果只发送给任务所有者对应的绑定；系统事件按绑定策略发送；Topic 绑定的通知直接进入指定 Topic。

Agent 调用由会话级队列和全局容量共同约束，不依赖 Telegram Dispatcher 的 Chat 级串行策略。较长响应会按 Telegram 消息长度安全拆分，未支持的媒体输入会被明确拒绝。

## 配置接口

Core 只保留以下会话鉴权接口：

- `GET /api/v1/bot`
- `PUT /api/v1/bot`
- `POST /api/v1/bot/test`
- `GET /api/v1/bot/users`

配置使用 `chat_bindings[]` 关联 Chat ID、可选 Topic ID、Telegram 发送者、面板用户、交互模式和通知事件。`GET /bot` 始终将 `token`、`cf_token` 返回为空，仅通过 `token_present`、`cf_token_present` 表示是否已配置；`PUT` 和测试请求中的空 secret 表示保留原值，只有 `clear_token` 或 `clear_cf_token` 才会明确清除。修改、测试和读取最小用户选项均要求管理员身份。

旧版 `admin_chat_id/events` 在首次加载时自动迁移到数据库中的首个管理员，并从后续持久化配置中移除。schema v3 会忽略并清理旧配置中的 `login_2fa` 字段，同时完整保留 Chat、Topic、面板用户、Telegram User ID 白名单和通知事件。`(chat_id, thread_id)` 必须唯一，绑定用户必须真实存在，通知事件和交互模式使用严格枚举。
