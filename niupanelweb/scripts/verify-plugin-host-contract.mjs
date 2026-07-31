import { readFileSync } from "node:fs";
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

const checks = [
  [host.includes('pluginHasCapability(app.capabilities, "ui.invoke")'), "host must gate invoke with ui.invoke"],
  [host.includes('pluginHasCapability(app.capabilities, "agents.invoke")'), "host must preserve agents.invoke compatibility"],
  [host.includes('capability.endsWith(".*")'), "host capability matching must support namespace wildcards"],
  [host.includes("`/plugins/${encodeURIComponent(app.plugin_id)}/invoke`"), "host must call the encoded native plugin invoke endpoint"],
  [routes.includes('route("/{id}/invoke", post(handlers::invoke_plugin_action))'), "backend invoke route is missing"],
  [handler.includes("plugin_ui_may_invoke(&record.manifest.capabilities)"), "backend invoke capability gate is missing"],
];

const failures = checks.filter(([passed]) => !passed).map(([, message]) => message);
if (failures.length > 0) {
  console.error(`Plugin host contract verification failed:\n- ${failures.join("\n- ")}`);
  process.exit(1);
}

console.log("Plugin host contract verification passed.");
