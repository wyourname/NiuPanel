# Agent App Plugin Template

This template is the recommended starting point for private agent modules that
need to behave like installed NiuPanel apps.

It includes:

- an `agents` process backend at `backend/main.js`
- the persistent `json_lines` protocol with MCP `tool_call` / `tool_result` support
- a native Vue UI mounted by the panel through `@niupanel/plugin-sdk`
- multiple plugin routes under `/plugins/agent-app-template`
- manifest-scoped permissions and `ui.api.allow` rules
- examples for `context.api.invoke()` and `context.api.request()`

Build the UI before installing the plugin directory:

```bash
cd examples/plugins/agents/app-template/ui
pnpm install
pnpm run build
```

Install from the panel with the server-side path:

```text
examples/plugins/agents/app-template
```

The UI entry is:

```text
ui/dist/niupanel-plugin.js
```

The backend entry is:

```text
backend/main.js
```

Invoke action `call_tool` with an optional `tool_input` object to exercise the
first MCP tool exposed by the panel. Normal actions continue to return a final
JSON-lines response without making a tool call.

Use this template when migrating closed-source agents into a private plugin
package. Keep private code in the plugin package, keep only the SDK contract and
manifest schema in the open panel repository.
