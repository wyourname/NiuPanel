# NiuPanel 插件开发

NiuPanel native UI plugins are trusted Vue applications mounted inside the panel shell.
They are not iframes and they run in the same browser context as the panel.

Use this mode only for first-party or trusted signed plugins. A
same-origin Vue module is not a browser security boundary; unknown third-party
UI code must not be installed as `vue_app`.

## Package Layout

```text
plugin.json
backend/run.sh
ui/dist/niupanel-plugin.js
ui/dist/assets/...
```

## Manifest UI Contract

```json
{
  "schema_version": 2,
  "id": "my-plugin",
  "name": "My Plugin",
  "version": "0.1.0",
  "runtime": "process",
  "entry": "backend/run.sh",
  "actions": [
    {
      "name": "query",
      "description": "Query plugin data",
      "callers": ["ui", "task", "api_key"],
      "timeout_sec": 30,
      "input_schema": {
        "type": "object",
        "properties": {
          "keyword": { "type": "string", "maxLength": 200 }
        },
        "required": ["keyword"],
        "additionalProperties": false
      }
    }
  ],
  "compatibility": {
    "panel": {
      "min_version": "0.8.0",
      "max_version": null
    },
    "dependencies": [
      {
        "id": "another-plugin",
        "min_version": "0.1.0",
        "optional": true
      }
    ]
  },
  "ui": {
    "enabled": true,
    "mode": "vue_app",
    "entry": "ui/dist/niupanel-plugin.js",
    "sdk_version": "^0.1.0",
    "display": {
      "sidebar": true,
      "workspace": true,
      "mobile": true,
      "category": "plugins",
      "order": 100,
      "layout": "panel"
    },
    "routes": [
      {
        "id": "home",
        "path": "/plugins/my-plugin",
        "title": "My Plugin",
        "description": "Plugin home page",
        "icon": "i-ep-box",
        "order": 0
      }
    ],
    "permissions": [],
    "api": {
      "allow": [
        {
          "methods": ["GET"],
          "path": "/tasks/**"
        }
      ]
    }
  }
}
```

## Compatibility And Dependencies

`compatibility.panel` declares the supported NiuPanel version range:

- `min_version`: minimum required panel version.
- `max_version`: maximum supported panel version.

`compatibility.dependencies` declares plugin dependencies:

- `id`: dependency plugin id.
- `min_version`: optional minimum dependency version.
- `optional`: when `false`, missing or too-old dependencies block install,
  update, and enable; when `true`, they appear as warnings.

Impact preview and health checks both evaluate compatibility. Required
dependency failures and incompatible panel versions are blockers. Optional
dependency failures are warnings.

## Runtime Capabilities

`capabilities` declares what runtime actions a plugin is allowed to handle. Names
must use lowercase dotted identifiers such as:

```json
{
  "capabilities": ["compiler.versions", "compiler.encrypt"]
}
```

Each capability must have at least two dot-separated segments. Segments may use
lowercase letters, digits, `_`, and `-`. A final wildcard segment such as
`compiler.*` is accepted for trusted plugins.

The compiler extension enforces these capabilities at runtime:

- `compiler.versions` is required before the panel calls the plugin for supported
  Python versions.
- `compiler.encrypt` is required before the panel calls the plugin for code
  encryption.

Missing compiler capabilities are shown as warnings in impact preview and health
checks; the plugin remains installable but will be skipped for the missing
runtime action.

## Compiler And Agents Extension Boundary

The panel treats compiler and agents behavior as plugin-owned. The `niupanel`
backend does not ship an in-repository compiler
implementation; `/api/v1/compiler/versions` and `/api/v1/compiler/encrypt`
require an enabled compiler plugin with these capabilities:

- `compiler.versions`
- `compiler.encrypt`

Compiler implementations belong in plugin apps that declare `compiler.*`
capabilities. Install them through the generic `/api/v1/plugins/install` or
`/api/v1/plugins/upload` endpoint, or through a configured plugin market.

Agents plugins use the same public plugin runtime and are installed under
`data/plugins/<plugin-id>` through the generic `/api/v1/plugins` lifecycle. The frontend exposes
`/plugins/agents` as the fixed agents app gateway: it loads the first enabled
plugin app that declares `agents.*` capabilities. A plugin can still expose
its own native route such as `/plugins/private-agent-app`; the fixed gateway is
reserved for the product-level Agents entry. Agent features use the same plugin
lifecycle and permission model as other extensions rather than a separate
`/api/v1/agents/*` business API.

Agent-specific sessions, memories, runs, and tool-call state are plugin-owned.
Plugins must not open or migrate the panel database. Persistent plugin state
is limited to non-database files under `NIUPANEL_PLUGIN_DATA_DIR`; panel business
data must be read or changed through an explicitly allowed host API or tool call.

Telegram 使用 Core 管理的可信传输通道调用 Agent Action。自然语言处理与面板操作属于
Agent 插件，Transport 不包含 Telegram 专属任务命令或 workflow；设计见
[Telegram Agent 通道](telegram-bot.md)。

## Public Release Gate

Before publishing an open panel artifact, run:

```bash
scripts/verify-public-release-gate.sh
```

The gate checks:

- private loader and agents source directories are absent from the public repo
- public Rust builds compile without private loader or agents crates
- the default backend dependency tree does not contain private loader or agents crates
- legacy private cfg/features and old Agents UI/API files are absent
- old agents-plugin compatibility route references are absent

## Plugin App Contract

`ui.display` controls where the host exposes the plugin app:

- `sidebar`: show the plugin in the panel navigation.
- `workspace`: allow the plugin to open as a desktop workspace window.
- `mobile`: allow the plugin route on mobile.
- `category`: lowercase grouping key for future app organization.
- `order`: lower values appear earlier.
- `layout`: `panel` for normal panel chrome, or `full_bleed` for immersive tools.

`ui.routes` declares pages owned by the plugin. The first visible route by
`order` is used as the default entry. A route may set `hidden: true` when it is
reachable only through plugin-internal navigation.

`ui.permissions` declares the panel permissions the plugin may request through
the API proxy. Every item must be an exact permission. Resource wildcards and
`*:*` are rejected.

`ui.api.allow` is a required allowlist for API access. An empty list denies every
proxied API request. Methods must be explicit; `*` and global path wildcards are
rejected. Every proxied request must match one rule before permission checks run.
Path rules support exact paths, trailing `*`, and `/**` prefixes:

```json
{
  "api": {
    "allow": [
      { "methods": ["GET"], "path": "/tasks/**" },
      { "methods": ["POST"], "path": "/compiler/**" }
    ]
  }
}
```

The panel exposes enabled plugin apps from:

```text
GET /api/v1/plugins/apps
```

The response is filtered by the current user's permissions. A plugin app is only
listed when every permission declared in `ui.permissions` is recognized by the
panel and granted to the current user.

The panel serves plugin UI assets from:

```text
GET /api/v1/plugins/{plugin_id}/ui/{asset_path}
```

Only files under the directory containing `ui.entry` are exposed as UI assets.
The asset endpoint applies the same `ui.permissions` check before serving files.

Plugin UI code should call panel APIs through the SDK:

```ts
const tasks = await context.api.request({
  method: "GET",
  path: "/tasks",
});
```

Frontend host code must normalize plugin routes through
`src/utils/pluginRoutes.ts`. This keeps direct routes, workspace windows, and
extension gateways using the same `/plugins/{plugin_id}/...` to
`context.route.path/query` mapping. The public release gate runs
`pnpm run verify:plugin-route-contract` to prevent local duplicate parsers from
drifting.

The host sends this request through:

```text
POST /api/v1/plugins/{plugin_id}/api
```

## Process Runtime Environment

Process plugins run with their plugin directory as the current working

- `NIUPANEL_PLUGIN_PROTOCOL`
- `NIUPANEL_PLUGIN_ID`
- `NIUPANEL_PLUGIN_EXTENSION`
- `NIUPANEL_PLUGIN_DATA_DIR`

Before `exec`, the child process clears the inherited environment and supplementary
groups, drops from root to the unprivileged `65534:65534` identity when available,
applies a Linux Landlock filesystem and TCP ruleset, and installs a baseline seccomp filter. It can read its
own package, write its own data directory, use its private `plugin-data/tmp`, and
access only the runtime libraries and system files required for execution and TLS.
The panel database, backups, scripts, logs, managed runtimes, session key, broad
`/etc` contents, process memory APIs, mount APIs, kernel APIs, and host environment
are not allowed. If Landlock is unavailable, process plugins fail closed and are
not started. This does not require container privileges or a setuid helper.

`plugin.json.env` may add plugin-owned configuration, but cannot override
`DATABASE_URL`, `NIUPANEL_*`, `PATH`, `HOME`, `TMPDIR`, shell startup variables,
or dynamic linker variables.

The sandbox has no network sockets by default. A plugin that must call an external
Web API must request it explicitly:

```json
{
  "runtime_permissions": ["network_outbound"]
}
```

On kernels with Landlock ABI v4, this permission allows outbound TCP connections
only to ports `80` and `443`. Older kernels keep the explicit outbound permission
but cannot enforce the port allowlist, so the install preview reports this possible
degradation.

A plugin that genuinely needs a user-configurable TCP port can instead request the
mutually exclusive high-risk permission:

```json
{
  "runtime_permissions": ["network_outbound_all_ports"]
}
```

This permits outbound TCP connections to any destination port, including services
on localhost and private networks. It does not grant panel API credentials or any
additional filesystem access. Panel business data remains accessible only through
the audited host API proxy and its manifest/user permission checks. Prefer an HTTPS
reverse proxy on port `443` when the upstream supports one.

The backend validates three things before forwarding the request to the internal
API:

1. The plugin is enabled and has a native Vue UI.
2. The plugin manifest declares the permission required by the requested API.
3. The current user has that permission.

For example, `GET /tasks` requires `task:list`, `GET /tasks/{id}/logs` requires
`task:read`, and `POST /compiler/encrypt` requires `compiler:run`.
The manifest must declare that exact permission.

The proxy rejects unknown paths. It permanently blocks raw file APIs, settings,
authentication, users, API keys, plugin management, Git configuration, terminal,
MCP administration, backup/restore, and system update routes.

Every allowed, denied, or failed plugin API proxy request and backend invocation
is written to the panel audit log. Audit details include the plugin id, action,
method, path, result, and status code, but never include the request body.

## Declarative Themes

A panel skin is a normal plugin using `runtime: "declarative"`. It has no
executable entry and may provide only validated theme tokens:

```json
{
  "id": "my-theme",
  "name": "My Theme",
  "version": "0.1.0",
  "description": "Panel color theme",
  "runtime": "declarative",
  "capabilities": ["ui.theme"],
  "theme": {
    "enabled": true,
    "light": {
      "primary": "#0F766E",
      "bg_base": "#F4F6F5",
      "bg_card": "#FFFFFF",
      "text_default": "#1B2422"
    },
    "dark": {
      "primary": "#5EEAD4",
      "bg_base": "#111715",
      "bg_card": "#18201E",
      "text_default": "#EDF5F2"
    }
  }
}
```

Theme values accept only six or eight digit hexadecimal colors. Theme packages
cannot inject JavaScript, arbitrary CSS, fonts, URLs, layout changes, or database
operations. The selected theme is stored in the browser, not in the panel database.

## Impact Preview And Route Governance

Before installing or updating a plugin from a server-side directory, the panel
can preview the app impact:

```text
POST /api/v1/plugins/preview
POST /api/v1/plugins/preview-upload
POST /api/v1/plugins/{id}/preview-update
POST /api/v1/plugins/{id}/preview-upload-update

POST /api/v1/plugins/market/preview
```

The preview returns target version, current version when present, UI routes,
permission additions/removals, `api.allow` additions/removals, warnings, route
conflicts, and blockers.

Path install/update, upload install/update, and market install/update use the
same impact check before writing plugin files. The operation is blocked when:

- A plugin is installed through the install endpoint but the same plugin id is
  already installed.
- An update package id does not match the target plugin id.
- A plugin UI route conflicts with another installed plugin route.

Use the update endpoint instead of reinstalling an existing plugin so version
history and rollback remain intact.

## Packaging And Plugin Markets

Plugin management uses `plugin.json` as the only package manifest.

To start a standalone private repository for closed-source plugins, generate a
workspace from the public templates:

```bash
node scripts/create-private-plugin-repo.mjs ../niupanel-private-plugins \
  --agents-id private-agent-app \
  --compiler-id private-compiler-loader
```

The generated repository contains:

- `plugins/private-agent-app`
- `plugins/private-compiler-loader`
- `packages/plugin-sdk`
- `schemas/plugin.schema.json`
- `scripts/package-plugin.mjs`
- `package.json` scripts for packaging both plugins into `dist/plugins`

Run the generated verification script before publishing a private market index:

```bash
cd ../niupanel-private-plugins
pnpm run verify
```

The generated manifests include:

```json
{
  "$schema": "../../../schemas/plugin.schema.json"
}
```

The public schema source is `docs/plugins/plugin.schema.json`. It is intended
for IDE completion and review automation; the backend still performs install
time validation and impact checks.

Use the repository packaging helper to validate a plugin directory, create a
`.tgz` package, compute its sha256 checksum, and optionally create or update a
market index:

```bash
node scripts/package-plugin.mjs examples/plugins/compiler/echo-compiler \
  --out dist/plugins \
  --market dist/plugins/index.json \
  --index-name "NiuPanel Private Plugin Market"
```

For Vue plugin apps, build the UI before packaging:

```bash
cd examples/plugins/agents/app-template/ui
pnpm install
cd -
node scripts/package-plugin.mjs examples/plugins/agents/app-template \
  --build-ui \
  --out dist/plugins \
  --market dist/plugins/index.json
```

The package contains one top-level directory named after the plugin id. The
helper excludes common development directories such as `.git`, `node_modules`,
`target`, `.idea`, and `.vscode`. A plugin may also define `.pluginignore` with
one relative path or glob per line to exclude private build sources from the
package. For closed-source binary plugins, build the binary into `bin/`, set the
manifest entry to a wrapper script such as `run.sh`, and ignore `backend/`.

The generated market index uses the platform-asset schema consumed by
`/api/v1/plugins/market`. Its package name includes the current platform, such
as `my-plugin-v0.1.0-linux-x64.tgz`; use `--platform` when packaging for a
different target. `checksum_sha256` is filled automatically. To sign packages,
pass an Ed25519 private key:

```bash
node scripts/generate-plugin-signing-key.mjs
node scripts/package-plugin.mjs examples/plugins/compiler/echo-compiler \
  --out dist/plugins \
  --market dist/plugins/index.json \
  --sign-key plugin-ed25519.pem
```

The command prints `trusted_key`. Put that value in
`TRUSTED_PLUGIN_PUBLIC_KEYS` on the server when `plugin_signature_required` is
enabled.

## Health Checks

The panel exposes plugin health reports from:

```text
GET /api/v1/plugins/health
```

Health checks validate the installed manifest state, plugin directory, backend
entry path, UI entry path, and enabled UI route conflicts. Enabling a plugin runs
the same health checks first and rejects the operation when any `error` check is
present. Warnings are shown in the management UI but do not block enabling.

## SDK Contract

Plugin UI modules should use `@niupanel/plugin-sdk`:

```ts
import { createApp } from "vue";
import { definePlugin } from "@niupanel/plugin-sdk";
import App from "./App.vue";

export default definePlugin({
  mount(el, context) {
    const app = createApp(App, { context });
    app.provide("niupanel", context);
    app.mount(el);
    return app;
  },
  unmount(app) {
    app.unmount();
  },
});
```

The host provides:

- `context.pluginId`
- `context.app`
- `context.route`
- `context.route.onChange()` for in-app route changes without remounting
- `context.api.request()`
- `context.api.invoke()` for native apps with a process backend
- `context.api.invokeStream()` for manifest-declared streaming actions
- `context.ui.toast()`
- `context.ui.confirm()`
- `context.ui.navigate()`

Native plugin apps that call their process backend through
`context.api.invoke()` must use `schema_version: 2` and declare the action with
`"ui"` in `actions[].callers`. `ui.invoke` and `agents.invoke` are not action
authorization; schema v1 packages that still use them must migrate before they
can be installed or updated.

```json
{
  "schema_version": 2,
  "runtime": "process",
  "protocol": "json_lines",
  "actions": [
    {
      "name": "chat",
      "callers": ["ui", "task", "api_key"],
      "timeout_sec": 120,
      "streaming": true,
      "input_schema": {
        "type": "object",
        "properties": {
          "message": { "type": "string", "minLength": 1 }
        },
        "required": ["message"],
        "additionalProperties": false
      }
    }
  ]
}
```

```ts
for await (const frame of context.api.invokeStream<{ message: string }>(
  "chat",
  { message: "检查失败任务" },
  { signal: abortController.signal },
)) {
  if (frame.type === "event" && frame.event === "delta") {
    renderDelta((frame.data as { text: string }).text);
  } else if (frame.type === "result") {
    renderFinal(frame.data.message);
  }
}
```

Streaming is opt-in per action with `"streaming": true` and requires
`protocol: "json_lines"`. For a streaming request the host adds `"stream": true`
to the process request. The plugin may emit ordered intermediate frames before
its normal final response:

```json
{"type":"stream_event","request_id":"...","sequence":1,"event":"delta","data":{"text":"..."}}
```

`request_id` must match, `sequence` must be positive and strictly increasing,
and event names use lowercase letters, digits, and underscores. The HTTP stream
maps these frames to named SSE events and then emits exactly one terminal
`result` or `error` event. Disconnecting the client cancels the invocation,
discards its worker, and releases its concurrency permits.

Each action has a manifest-owned timeout between 1 and 180 seconds. Callers
cannot override it. Request bodies, process output, and each JSON Lines frame
are limited to 1 MiB. The runtime allows at most 16 concurrent plugin calls
globally and at most `worker.max` calls per plugin.

## Task And External API Calls

Task scripts use the bundled SDK and their injected internal token. External
clients use an API Key with the exact `plugin:invoke` permission. In both cases,
the host first filters actions by `actions[].callers` and then validates input
against `input_schema`.

```python
from niu import niu

plugins = niu.list_plugins()
actions = niu.list_plugin_actions("my-plugin")
result = niu.invoke_plugin("my-plugin", "query", {"keyword": "test"})
```

```js
const niu = require("niu");

const plugins = await niu.listPlugins();
const actions = await niu.listPluginActions("my-plugin");
const result = await niu.invokePlugin("my-plugin", "query", { keyword: "test" });
```

The corresponding Open API endpoints are:

```text
GET  /open/api/plugins
GET  /open/api/plugins/{plugin_id}/actions
POST /open/api/plugins/{plugin_id}/invoke
POST /open/api/plugins/{plugin_id}/invoke/stream
```

The invoke body is `{ "action": "query", "input": { ... } }`. Discovery only
returns plugins and actions callable by the current token (`task` for an
injected SDK token, `api_key` for an external API Key, or `telegram` for the
Core-managed Telegram Agent channel).

External clients consume the streaming endpoint with normal POST SSE semantics:

```bash
curl -N https://panel.example.com/open/api/plugins/my-plugin/invoke/stream \
  -H 'X-API-Key: <api-key>' \
  -H 'Accept: text/event-stream' \
  -H 'Content-Type: application/json' \
  --data '{"action":"chat","input":{"message":"检查失败任务"}}'
```

## Template

Start from:

```text
examples/plugins/vue-app-template
```

For private agent modules that should look and behave like installed apps, start
from:

```text
examples/plugins/agents/app-template
```

That template combines an `agents` process backend with a native Vue workspace,
multiple routes, `context.api.invoke()`, panel API proxy access, and a minimal
permission/API allow-list.

Build its UI before installing the plugin package:

```bash
cd examples/plugins/vue-app-template/ui
pnpm install
pnpm run build
```

For the agent app template:

```bash
cd examples/plugins/agents/app-template/ui
pnpm install
pnpm run build
```

Then install the package directory from the panel:

```text
examples/plugins/vue-app-template
```

or:

```text
examples/plugins/agents/app-template
```

## Agent App Migration Path

Use the agent app template for closed-source `agents` modules that need their
own panel pages:

1. Keep the public panel repository limited to the SDK, manifest schema, API
   proxy, and plugin host.
2. Move private agent logic into the plugin process entry, for example
   `backend/main.js` or a compiled binary.
3. Put native Vue pages under `ui/src` and build them to
   `ui/dist/niupanel-plugin.js`.
4. Declare all routes in `ui.routes`; use hidden routes for pages that should be
   reachable only from inside the plugin.
5. Declare the smallest possible `ui.permissions` and `ui.api.allow` surface.
6. Publish the package through a private market index or upload a signed archive.

The public `/agents` panel entry is intentionally a gateway. It loads the first
available native Vue plugin app that declares `agents.*` capabilities,
preferring apps with `ui.display.category: "agents"`. If no agents plugin app is
installed or available to the current user, the gateway shows an install prompt.

This keeps the open panel repository responsible for navigation, permissions,
SDK contracts, and plugin hosting, while the private agents package owns the
actual product UI and agent-specific workflows.

Inside plugin UI code, use `context.route.onChange()` to react to internal route
changes. The host keeps the same Vue app mounted when only the plugin route path
or query changes:

```ts
const stop = context.route.onChange((route) => {
  activePath.value = route.path;
});
```

Call `stop()` from the plugin's component unmount hook when using framework-level
subscriptions.

On mobile and direct browser routes, plugin navigation updates the Vue Router
URL. In desktop workspace windows, plugin navigation updates the window payload
instead; the browser URL stays on the current panel page, but
`context.route.path`, `context.route.query`, and `context.route.onChange()` keep
the same semantics for plugin code.

## Packaging And Signing CLI

Generate an Ed25519 plugin signing key:

```bash
niupanel plugin keygen plugin-ed25519.pem plugin-ed25519.pub
```

The command prints a `sha256:<fingerprint>` value. Put that value in
`TRUSTED_PLUGIN_PUBLIC_KEYS` on the server.

Create a package without a signature:

```bash
niupanel plugin pack examples/plugins/vue-app-template dist/vue-app-template.tgz
```

Create and sign a package:

```bash
niupanel plugin pack examples/plugins/vue-app-template dist/vue-app-template.tgz --key plugin-ed25519.pem
```

The command prints metadata for a signed market entry or an API client:

```text
checksum_sha256: ...
signature_ed25519: ...
public_key_ed25519: ...
trusted_key: sha256:...
```

The output package path must not be inside the plugin source directory.

## Plugin Market Index

A plugin market is a static JSON file. It can be hosted by any internal HTTP
server, object storage bucket, or Git raw endpoint.

```json
{
  "schema_version": 1,
  "name": "Private NiuPanel Plugins",
  "description": "Internal plugin source",
  "signing": {
    "algorithm": "ed25519",
    "trusted_key": "sha256:public-key-fingerprint",
    "public_key_ed25519": "PEM, base64 raw Ed25519 public key, or hex raw Ed25519 public key"
  },
  "plugins": [
    {
      "id": "my-plugin",
      "name": "My Plugin",
      "version": "0.1.0",
      "description": "Internal plugin",
      "permissions": ["compiler:read", "compiler:run"],
      "assets": [
        {
          "platform": "linux-x64",
          "file": "./my-plugin-v0.1.0-linux-x64.tgz",
          "checksum_sha256": "64-character-sha256",
          "signature_ed25519": "base64-detached-signature"
        }
      ],
      "homepage": null,
      "repository": null
    }
  ]
}
```

The root `signing` object carries the shared Ed25519 public key. Each asset
must use a `platform` supported by the target server, such as `linux-x64`,
`linux-arm64`, or `linux-arm32`; its `file` can be absolute or relative to the
index URL. The legacy `download_url` entry format is not supported.

Management APIs:

```text
GET  /api/v1/plugins/market?index_url=https://example.com/plugins/index.json
GET  /api/v1/plugins/market/sources
PUT  /api/v1/plugins/market/sources
GET  /api/v1/plugins/market/updates
POST /api/v1/plugins/market/install
```

`/market/sources` stores the panel-side list of plugin publishing sources in
system settings. Each source has `name`, `url`, and `enabled`; URLs must use
HTTP or HTTPS and duplicate URLs are deduplicated by the backend.

`/market/updates` checks enabled publishing sources against installed
`agents` and `compiler` plugins and returns update records with the source,
installed version, available version, and market entry.

The install endpoint downloads the package, verifies `checksum_sha256` and
Ed25519 signature metadata when present or required by server config, then
installs or updates the target plugin. Existing plugins are updated through the
same version-history path, so rollback remains available.

Example index:

```text
examples/plugins/market/index.json
```

## Package Upload Integrity

Uploaded plugin packages support `.zip`, `.tar`, `.tar.gz`, and `.tgz`.
The package must contain `plugin.json` at the archive root or inside exactly one
top-level directory.

For uploaded packages, the panel accepts an optional form field:

```text
checksum_sha256=<64-character lowercase or uppercase hex digest>
```

When this field is present, the backend computes the SHA-256 digest of the
uploaded archive before extraction and rejects the package if it does not match.

Package extraction rejects absolute paths, parent-directory traversal, symlinks,
and hard links. Only regular files and directories are extracted.

Packages downloaded from a remote plugin market support optional detached
Ed25519 signatures. Unsigned market packages are accepted by default. To require
signatures, configure trusted public keys on the server and opt in explicitly:

```env
PLUGIN_SIGNATURE_REQUIRED=true
TRUSTED_PLUGIN_PUBLIC_KEYS=sha256:<raw-public-key-sha256>
```

When `PLUGIN_SIGNATURE_REQUIRED=true`, every package downloaded from a market
must include:

```text
signature_ed25519=<base64 detached signature over the uploaded archive bytes>
public_key_ed25519=<PEM, base64 raw Ed25519 public key, or hex raw public key>
```

The backend verifies that the submitted public key matches one configured in
`TRUSTED_PLUGIN_PUBLIC_KEYS`, then verifies the signature over the exact archive
bytes before extraction. A submitted public key is not trusted by itself. When
`PLUGIN_SIGNATURE_REQUIRED=false`, unsigned packages are accepted, but packages
that submit signing metadata are still verified with the same trust policy.

Direct package uploads from the authenticated administrator are treated as an
explicit local trust action. The upload dialog only needs the package file and
an optional SHA-256 checksum; it no longer asks for `signature_ed25519` or
`public_key_ed25519`. Low-level API clients may still provide both fields, in
which case they are verified normally.

Server-side path installation follows the same explicit local administrator
trust model. Native UI packages downloaded from a market remain subject to the
configured signature policy.

## Version History And Rollback

When a local plugin is updated from a server-side path or an uploaded package,
the previous active plugin directory is moved into the plugin history directory:

```text
data/plugins/.history/<plugin_id>/<timestamp>-<version>
```

The active plugin state records these archived versions in `.state.json`.

Management APIs:

```text
GET  /api/v1/plugins/{id}/versions
POST /api/v1/plugins/{id}/rollback/{version_id}
```

Rollback restores the selected archived version as the active plugin and archives
the previously active version, so rollback is reversible. If a process plugin is
running, the backend stops its worker pool after rollback so the next invocation
uses the restored version.

Built-in plugins cannot be updated, uninstalled, or rolled back.
# Real-host UI Development

Run the panel core separately and install/enable the plugin once. From the
NiuPanel repository root, start the Web UI with local source overrides:

```bash
node scripts/dev-workspace.mjs \
  --plugin ../NiuPanelPrivatePlugins/plugins/agents/niupanel-private-agents \
  --plugin ../NiuPanelPrivatePlugins/plugins/yyb-protocol \
  --api http://127.0.0.1:7788 --port 7787
```

Open `http://localhost:7787`, log in, and open the installed plugin normally.
Each directory must contain `plugin.json` and `ui/src/plugin.ts`. Install the
Web and plugin UI dependencies first. Repeat `--plugin` for multiple plugins.
The equivalent Web package command is `pnpm dev:workspace`; directory arguments
are relative to the current working directory.

The panel's Vite server compiles these source entries, including Vue SFCs and
CSS, and provides a single same-origin HMR connection. No package rebuild or
reinstallation is needed for UI edits. Vue template/style updates use HMR;
script updates may recreate components, and entry changes may reload the page.
Do not assume unsent inputs survive every type of update. Plugin components must
clean up timers, subscriptions and requests on unmount.

API calls still use the real injected SDK, logged-in identity and installed
plugin permissions. This override does not install a plugin, change its backend,
or grant new permissions. Backend and manifest edits still require a rebuild
and installation/reload through the existing plugin lifecycle. Automatic backend
replacement and manifest synchronization are not part of this UI development mode.

The launcher defaults to loopback and fails when the chosen port is occupied.
For a remote machine, forward this single port over SSH (recommended), or use
`--host 0.0.0.0` only on a trusted network. The Vite server exposes source code
without panel authentication and must not be publicly exposed. Browser URLs and
WebSockets use the panel origin, not the backend machine's `localhost`.

Normal `pnpm dev` has no overrides unless `NIUPANEL_DEV_PLUGINS` is explicitly set
to a JSON array of directories. Release builds ignore this variable entirely.
Stop the workspace server to stop source overrides; installed plugin files are
never modified. This mode currently supports the Vue/TypeScript entry convention
above and uses the host's Vue and SDK. Plugin-specific Vite transforms are not
automatically imported.
