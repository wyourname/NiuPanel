import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const files = [
  "src/stores/workspace.ts",
  "src/views/modules/AgentsGateway.vue",
];

const failures = [];

for (const relativePath of files) {
  const content = readFileSync(resolve(root, relativePath), "utf8");
  if (!content.includes("@/utils/pluginRoutes")) {
    failures.push(`${relativePath} must import shared plugin route utilities`);
  }
  if (/const\s+queryFromSearch\s*=/.test(content)) {
    failures.push(`${relativePath} must not redeclare queryFromSearch`);
  }
  if (/const\s+normalizePluginRoute\s*=/.test(content)) {
    failures.push(`${relativePath} must not redeclare normalizePluginRoute`);
  }
}

if (failures.length > 0) {
  console.error(`Plugin route contract verification failed:\n- ${failures.join("\n- ")}`);
  process.exit(1);
}

console.log(`Plugin route contract verification passed (${files.length} files).`);
