import { createHash } from "node:crypto";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const dist = resolve(root, "dist");
const assets = join(dist, "assets");
const sidebarSource = join(root, "src/layout/components/TheSidebar.vue");
const moreSource = join(root, "src/views/modules/more/index.vue");
const routerSource = join(root, "src/router/index.ts");
const workspaceAppsSource = join(root, "src/workspace/apps.ts");
const workspaceComponentsSource = join(root, "src/workspace/components.ts");
const shareSource = join(root, "src/views/modules/share/index.vue");
const extensionsSource = join(root, "src/views/modules/extensions/index.vue");
const apiKeyTabSource = join(root, "src/views/modules/settings/ApiKeyTab.vue");
const packageSource = join(root, "package.json");
const versionContractSource = join(root, "../niupanel-common/src/version.rs");
const releaseManifestSource = join(dist, "release-manifest.json");

const fail = (message) => {
  console.error(`Public frontend build verification failed: ${message}`);
  process.exit(1);
};

if (!existsSync(dist) || !existsSync(assets)) {
  fail("dist/assets does not exist; run pnpm run build first.");
}

if (existsSync(join(dist, "monacoeditorwork"))) {
  fail("legacy vite-plugin-monaco-editor worker output is present.");
}

const files = readdirSync(assets);
const workerPayloads = files.filter(
  (file) =>
    /^(?:editor|json|css|html|ts)\.worker-.*\.js$/.test(file) &&
    statSync(join(assets, file)).size > 1024,
);
if (workerPayloads.length !== 5) {
  fail(`expected one native Monaco worker set, found: ${workerPayloads.join(", ")}`);
}
const agentChunks = files.filter((file) => /^Agents[-.].*\.js$/.test(file));
if (agentChunks.length > 0) {
  fail(`legacy Agents chunks are present: ${agentChunks.join(", ")}`);
}

const forbiddenTexts = [
  "任务运维与确认式自动化",
  "Agent 验收",
  "学习闭环检查",
  "记忆治理检查",
];

const searchableFiles = [
  ...files.map((file) => join(assets, file)),
  join(dist, "index.html"),
].filter((file) => existsSync(file));

const indexHtml = readFileSync(join(dist, "index.html"), "utf8");
const initialAssetPaths = Array.from(
  indexHtml.matchAll(/(?:src|href)=["']\/?(assets\/[^"']+)["']/g),
  (match) => match[1],
);
const forbiddenInitialAssets = initialAssetPaths.filter((file) =>
  /monaco|worker|overview|terminal|capacitor|vendor-charts/i.test(file),
);
if (forbiddenInitialAssets.length > 0) {
  fail(`heavy feature chunks are preloaded on first paint: ${forbiddenInitialAssets.join(", ")}`);
}

const initialFiles = ["index.html", ...initialAssetPaths];
const initialRawBytes = initialFiles.reduce(
  (total, file) => total + statSync(join(dist, file)).size,
  0,
);
const initialGzipBytes = initialFiles.reduce((total, file) => {
  const rawFile = join(dist, file);
  const gzipFile = `${rawFile}.gz`;
  return total + statSync(existsSync(gzipFile) ? gzipFile : rawFile).size;
}, 0);
if (initialRawBytes > 3 * 1024 * 1024) {
  fail(`first paint raw assets exceed 3 MiB: ${(initialRawBytes / 1024 / 1024).toFixed(2)} MiB`);
}
if (initialGzipBytes > 900 * 1024) {
  fail(`first paint gzip assets exceed 900 KiB: ${(initialGzipBytes / 1024).toFixed(0)} KiB`);
}

const walkFiles = (directory) =>
  readdirSync(directory).flatMap((name) => {
    const path = join(directory, name);
    return statSync(path).isDirectory() ? walkFiles(path) : [path];
  });
const distFiles = walkFiles(dist);
const rawDistBytes = distFiles
  .filter((file) => !file.endsWith(".gz"))
  .reduce((total, file) => total + statSync(file).size, 0);
if (rawDistBytes > 18 * 1024 * 1024) {
  fail(`raw dist exceeds 18 MiB: ${(rawDistBytes / 1024 / 1024).toFixed(2)} MiB`);
}

const compressible = /\.(?:css|html|js|json|map|svg|ttf|txt|wasm|xml)$/;
const missingGzip = distFiles.filter(
  (file) =>
    !file.endsWith(".gz") &&
    file !== releaseManifestSource &&
    statSync(file).size >= 1024 &&
    compressible.test(file) &&
    !existsSync(`${file}.gz`),
);
if (missingGzip.length > 0) {
  fail(`precompressed gzip assets are missing: ${missingGzip.slice(0, 5).join(", ")}`);
}

const hits = [];
for (const file of searchableFiles) {
  const content = readFileSync(file, "utf8");
  for (const text of forbiddenTexts) {
    if (content.includes(text)) {
      hits.push(`${file}: ${text}`);
    }
  }
}

if (hits.length > 0) {
  fail(`legacy Agents UI text leaked into dist:\n${hits.join("\n")}`);
}

const sidebar = readFileSync(sidebarSource, "utf8");
for (const id of ['"agents"', '"compiler"']) {
  if (sidebar.includes(id)) {
    fail(`private extension app ${id} must not be registered as a built-in dock item.`);
  }
}

const more = readFileSync(moreSource, "utf8");
if (more.includes('name: "compiler"') || more.includes('path: "/compiler"')) {
  fail("compiler must not be registered as a built-in More menu item.");
}

const router = readFileSync(routerSource, "utf8");
if (/path:\s*['"]agents['"]/.test(router) || /path:\s*['"]compiler['"]/.test(router)) {
  fail("private extension apps must not be registered as built-in routes.");
}

const workspaceApps = readFileSync(workspaceAppsSource, "utf8");
if (/routeName:\s*["']agents["']/.test(workspaceApps) || /routeName:\s*["']compiler["']/.test(workspaceApps)) {
  fail("private extension apps must not be registered as built-in workspace apps.");
}

const workspaceComponents = readFileSync(workspaceComponentsSource, "utf8");
if (/\bagents:\s*AgentsGateway\b/.test(workspaceComponents) || /\bcompiler:\s*Compiler\b/.test(workspaceComponents)) {
  fail("private extension apps must not be registered as built-in workspace components.");
}

const share = readFileSync(shareSource, "utf8");
if (/McpPanel|PluginsPanel|activeTab\s*===\s*["'](?:mcp|plugins)["']/.test(share)) {
  fail("share center must only contain content distribution views.");
}

const extensions = readFileSync(extensionsSource, "utf8");
if (extensions.includes("McpPanel") || !extensions.includes("ExtensionManager")) {
  fail("extension center must only own plugin management views.");
}
if (!router.includes("name: 'extensions'") || !workspaceApps.includes('id: "extensions"')) {
  fail("extension center must be registered in router and workspace apps.");
}

const apiKeyTab = readFileSync(apiKeyTabSource, "utf8");
if (!apiKeyTab.includes("McpAccessPanel")) {
  fail("panel MCP access must be presented in API access settings.");
}

if (!existsSync(releaseManifestSource)) {
  fail("dist/release-manifest.json is missing.");
}
const releaseManifest = JSON.parse(readFileSync(releaseManifestSource, "utf8"));
const frontendPackage = JSON.parse(readFileSync(packageSource, "utf8"));
const versionContract = readFileSync(versionContractSource, "utf8");
const minimumCoreVersion = versionContract.match(
  /pub const MINIMUM_WEB_CORE_VERSION: &str = "([^"]+)";/,
)?.[1];
if (
  releaseManifest.component !== "web" ||
  releaseManifest.version !== frontendPackage.version ||
  releaseManifest.api_contract !== 1 ||
  !minimumCoreVersion ||
  releaseManifest.core?.min !== minimumCoreVersion
) {
  fail("Web release manifest contract is invalid.");
}
for (const [relativePath, expectedHash] of Object.entries(releaseManifest.files || {})) {
  const file = join(dist, relativePath);
  if (!existsSync(file)) fail(`Web release manifest references missing file: ${relativePath}`);
  const actualHash = createHash("sha256").update(readFileSync(file)).digest("hex");
  if (actualHash !== expectedHash) fail(`Web release checksum mismatch: ${relativePath}`);
}
if (Object.keys(releaseManifest.files || {}).length === 0) {
  fail("Web release manifest does not contain file checksums.");
}

console.log(
  `Public frontend build verification passed (first paint ${(initialRawBytes / 1024).toFixed(0)} KiB raw / ${(initialGzipBytes / 1024).toFixed(0)} KiB gzip, dist ${(rawDistBytes / 1024 / 1024).toFixed(2)} MiB raw).`,
);
