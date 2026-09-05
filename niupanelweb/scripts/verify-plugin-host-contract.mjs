import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

const webRoot = resolve(import.meta.dirname, "..");
const repositoryRoot = resolve(webRoot, "..");
const host = readFileSync(
  resolve(webRoot, "src/views/plugins/PluginHostView.vue"),
  "utf8",
);
const routes = readFileSync(
  resolve(repositoryRoot, "niupanel/src/modules/plugins/routes.rs"),
  "utf8",
);
const handler = readFileSync(
  resolve(repositoryRoot, "niupanel/src/modules/plugins/handlers/ui.rs"),
  "utf8",
);
const telegramRoutes = readFileSync(
  resolve(repositoryRoot, "niupanel/src/modules/telegram/routes.rs"),
  "utf8",
);
const manifest = readFileSync(
  resolve(repositoryRoot, "niupanel-plugin/src/manifest.rs"),
  "utf8",
);
const workspaceApps = readFileSync(
  resolve(webRoot, "src/workspace/apps.ts"),
  "utf8",
);

const checks = [
  [host.includes("(app.actions ?? []).some"), "host must require a declared action"],
  [host.includes("declaredAction.name === action"), "host must match the requested action by name"],
  [host.includes("`/plugins/${encodeURIComponent(app.plugin_id)}/invoke`"), "host must call the encoded native plugin invoke endpoint"],
  [host.includes("/invoke/stream") && host.includes("invokePluginActionStream"), "host streaming invoke implementation is missing"],
  [host.includes("options.signal") && host.includes("reader.read()"), "host streaming invoke must support cancellation and incremental reads"],
  [routes.includes('"/{id}/invoke"') && routes.includes("post(handlers::invoke_plugin_action)"), "backend invoke route is missing"],
  [routes.includes('"/{id}/invoke/stream"') && routes.includes("post(handlers::stream_plugin_action)"), "backend streaming invoke route is missing"],
  [routes.includes("MAX_PLUGIN_INVOCATION_BODY_BYTES"), "backend invoke body limit is missing"],
  [handler.includes("PluginActionCaller::Ui") && handler.includes(".invoke_action_with_tools_context("), "backend must enforce the UI action caller and trusted invocation context through the tool gateway"],
  [handler.includes("AgentToolGateway::for_plugin("), "backend must resolve trusted host tools for the plugin"],
  [manifest.includes("pub callers: Vec<PluginActionCaller>"), "action caller declarations are missing"],
  [manifest.includes("pub streaming: bool"), "action streaming declaration is missing"],
  [manifest.includes("Telegram"), "Telegram plugin action caller is missing"],
  [host.includes('first === "bot"') && host.includes('"setting:update"'), "plugin host Telegram settings permission mapping is missing"],
  [!workspaceApps.includes('id: "telegram"'), "legacy Telegram workspace app must stay hidden"],
  [!telegramRoutes.includes("/commands") && !telegramRoutes.includes("/workflows"), "legacy Telegram command and workflow routes must stay removed"],
  [!existsSync(resolve(repositoryRoot, "niupanel/src/modules/telegram/commands_api.rs")), "legacy Telegram commands API must stay removed"],
  [!existsSync(resolve(repositoryRoot, "niupanel/src/modules/telegram/workflows_api.rs")), "legacy Telegram workflows API must stay removed"],
  [!host.includes('pluginHasCapability(app.capabilities, "ui.invoke")'), "host must not use the legacy ui.invoke capability gate"],
];

const failures = checks.filter(([passed]) => !passed).map(([, message]) => message);
if (failures.length > 0) {
  console.error(`Plugin host contract verification failed:\n- ${failures.join("\n- ")}`);
  process.exit(1);
}

console.log("Plugin host contract verification passed.");
