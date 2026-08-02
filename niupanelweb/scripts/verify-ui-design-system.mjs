import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (relativePath) => readFileSync(join(root, relativePath), "utf8");
const fail = (message) => {
  console.error(`UI design system verification failed: ${message}`);
  process.exit(1);
};

const packageJson = JSON.parse(read("package.json"));
const pnpmLock = read("pnpm-lock.yaml");
if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(packageJson.version)) {
  fail(`invalid frontend version: ${packageJson.version}`);
}
if (packageJson.version === "0.0.0") {
  fail("frontend version must not remain at 0.0.0");
}
if (!packageJson.packageManager?.startsWith("pnpm@")) {
  fail("package.json must pin pnpm through packageManager");
}
if (!pnpmLock.includes("lockfileVersion:") || !pnpmLock.includes("importers:")) {
  fail("pnpm-lock.yaml is missing or invalid");
}

const viteConfig = read("vite.config.ts");
const versionModule = read("src/version.ts");
if (!viteConfig.includes("__APP_VERSION__") || !viteConfig.includes("frontendPackage.version")) {
  fail("Vite must inject the package version as __APP_VERSION__");
}
if (!versionModule.includes("FRONTEND_VERSION = __APP_VERSION__")) {
  fail("src/version.ts must expose the injected frontend version");
}

const globalCss = read("src/assets/styles/index.css");
for (const token of [
  "--font-size-xs: 11px",
  "--font-size-sm: 12px",
  "--font-size-body: 14px",
  "--font-size-title: 16px",
  "--font-size-page: 20px",
  "--space-1: 4px",
  "--control-sm: 32px",
  "--control-md: 36px",
  "--control-lg: 40px",
  "--mobile-dock-clearance:",
  "--brand-primary-rgb:",
  "--radius-sm: 6px",
  "--radius-md: 8px",
  "--accent-subtle-bg:",
  "--accent-subtle-text:",
  "--accent-subtle-border:",
  "--success-subtle-bg:",
  "--warning-subtle-bg:",
  "--danger-subtle-bg:",
]) {
  if (!globalCss.includes(token)) fail(`missing shared token: ${token}`);
}

const guardedFiles = [
  "src/assets/styles/index.css",
  "uno.config.ts",
  "src/components/workspace/DesktopWorkspaceHome.vue",
  "src/components/workspace/WorkspaceAppFrame.vue",
  "src/components/workspace/WorkspaceWindow.vue",
  "src/layout/components/TheSidebar.vue",
  "src/layout/components/DesktopStatusBar.vue",
  "src/components/tasks/TaskDesktopNavigatorHeader.vue",
  "src/components/tasks/TaskDetailHeader.vue",
  "src/components/tasks/TaskInfoPanel.vue",
  "src/components/tasks/TaskMobileListPane.vue",
  "src/views/modules/more/index.vue",
  "src/views/modules/settings/index.vue",
  "src/views/modules/settings/components/AboutOverviewCard.vue",
];
const forbiddenPatterns = [
  [/(?:bg-gradient|linear-gradient|radial-gradient)/, "gradient decoration"],
  [/backdrop-blur/, "backdrop blur"],
  [/tracking-(?:tight|wide|wider|widest)/, "non-standard letter spacing"],
  [/rounded-(?:2xl|3xl)|rounded-\[[^\]]+\]/, "oversized arbitrary radius"],
];

for (const file of guardedFiles) {
  const source = read(file);
  for (const [pattern, description] of forbiddenPatterns) {
    if (pattern.test(source)) fail(`${file} contains ${description}`);
  }
}

const shortcuts = read("uno.config.ts");
if (!shortcuts.includes("'bg-subtle': 'bg-[var(--bg-subtle)]'")) {
  fail("UnoCSS must expose the bg-subtle semantic shortcut");
}
if (!shortcuts.includes("'accent-subtle':")) {
  fail("UnoCSS must expose the paired accent-subtle state shortcut");
}
if (!shortcuts.includes("primary: 'rgb(var(--brand-primary-rgb) / %alpha)'")) {
  fail("UnoCSS primary color must preserve opacity modifiers such as bg-primary/10");
}
if (/transition-all|active:scale|hover:-translate/.test(shortcuts)) {
  fail("shared shortcuts must not use layout-shifting interaction effects");
}

const pluginThemes = read("src/stores/pluginThemes.ts");
if (!pluginThemes.includes('"--brand-primary-rgb": rgbChannels(primary)')) {
  fail("plugin themes must update the RGB primary token used by opacity modifiers");
}

const bulkActionBar = read("src/components/common/BulkActionBar.vue");
if (/absolute[^\n]*-translate-x-1\/2/.test(bulkActionBar)) {
  fail("desktop bulk actions must use normal document flow instead of a centered overlay");
}

const taskDetailCommandRouter = read("src/composables/useTaskDetailCommandRouter.ts");
if (!/case "edit_script":[\s\S]*?activeDetailTab\.value = "script";/.test(taskDetailCommandRouter)) {
  fail("desktop edit-script command must switch to the script detail tab");
}

const taskDetailHeader = read("src/components/tasks/TaskDetailHeader.vue");
if (
  taskDetailHeader.includes("open-log-window") ||
  taskDetailHeader.includes("打开日志窗口")
) {
  fail("task detail header must not expose the redundant open-log-window action");
}
if (
  taskDetailHeader.indexOf('aria-label="搜索日志"') === -1 ||
  taskDetailHeader.indexOf('aria-label="搜索日志"') >
    taskDetailHeader.indexOf('<nav class="hidden lg:flex')
) {
  fail("task log search must appear before the desktop detail tabs");
}

const responsiveDialog = read("src/components/common/ResponsiveDialog.vue");
if (!responsiveDialog.includes(':show-close="false"')) {
  fail("mobile responsive drawers with a custom header must disable the built-in close button");
}

const taskMobileListPane = read("src/components/tasks/TaskMobileListPane.vue");
if (taskMobileListPane.includes("<FloatingActionButton")) {
  fail("mobile Tasks must place create next to search instead of using a floating action button");
}
if (!taskMobileListPane.includes('aria-label="新建任务"')) {
  fail("mobile Tasks search toolbar must expose an accessible create button");
}

const taskCardContent = read("src/components/tasks/TaskCardContent.vue");
const taskCardItem = read("src/components/tasks/TaskCardItem.vue");
if (!taskCardContent.includes("下次 {{ nextRunText }}")) {
  fail("mobile task rows must display the next scheduled run");
}
if (!taskCardContent.includes("v-if=\"task.status !== 'Failed'\"")) {
  fail("mobile failed status must be moved out of the task title row");
}
if (
  !taskCardContent.includes('(event: "logs"): void;') ||
  !taskCardContent.includes('emit("logs");') ||
  !taskCardItem.includes('@logs="emit(\'logs\', task)"')
) {
  fail("task primary log action must emit logs directly through the card event chain");
}
const taskPrimaryActionHandler =
  taskCardContent.match(
    /const handlePrimaryAction = \(\) => \{[\s\S]*?\n\};/,
  )?.[0] ?? "";
if (
  !taskPrimaryActionHandler ||
  taskPrimaryActionHandler.includes('emit("more-actions");')
) {
  fail("task primary action must not route view-log clicks through the more-actions menu");
}
if (
  !taskCardContent.includes('class="h-11 rounded-md') ||
  !taskCardContent.includes('class="h-11 w-11 rounded-md')
) {
  fail("mobile task primary and more actions must preserve 44px touch targets");
}

for (const file of [
  "src/components/workspace/WorkspaceLayer.vue",
  "src/views/modules/environment/components/EnvTable.vue",
  "src/views/modules/more/index.vue",
  "src/views/modules/settings/index.vue",
  "src/views/modules/settings/components/ApiKeyList.vue",
]) {
  const source = read(file);
  if (/\bpb-(?:20|24)\b/.test(source)) {
    fail(`${file} must use mobile-dock-safe instead of a hard-coded Dock offset`);
  }
}

if (read("src/views/modules/more/index.vue").includes('name: "share"')) {
  fail("the More page must not duplicate the mobile Dock share entry");
}

const taskMobileList = read("src/components/tasks/TaskMobileListPane.vue");
if (taskMobileList.includes("@click=\"emit('create')\"")) {
  fail("mobile Tasks must use one create entry instead of duplicating the floating action button");
}

for (const file of guardedFiles) {
  const source = read(file);
  if (/bg-primary\/(?:5|10|15|20)\s+text-primary/.test(source)) {
    fail(`${file} uses an unsafe primary-on-primary state; use accent-subtle`);
  }
}

console.log(`UI design system verified. Web version: ${packageJson.version}`);
