import { readdirSync, readFileSync, statSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const read = (relativePath) => readFileSync(resolve(root, relativePath), "utf8");
const failures = [];

const check = (condition, message) => {
  if (!condition) failures.push(message);
};

const walk = (directory) =>
  readdirSync(directory).flatMap((entry) => {
    const path = resolve(directory, entry);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });

const sourceFiles = walk(resolve(root, "src")).filter((path) =>
  /\.(?:css|ts|vue)$/.test(path),
);

const uno = read("uno.config.ts");
const responsiveConstants = read("src/constants/responsive.ts");
const appStore = read("src/stores/app.ts");
const viewport = read("index.html");
const globalCss = read("src/assets/styles/index.css");
const mainLayout = read("src/layout/MainLayout.vue");
const header = read("src/layout/components/TheHeader.vue");
const dock = read("src/layout/components/TheSidebar.vue");
const pluginApps = read("src/stores/pluginApps.ts");
const responsiveDialog = read("src/components/common/ResponsiveDialog.vue");
const overlayDrawer = read("src/components/common/OverlayDrawer.vue");
const overlayFooter = read("src/components/common/OverlayFooter.vue");
const overlayHeader = read("src/components/common/OverlayHeader.vue");
const environmentView = read("src/views/modules/Environment.vue");
const environmentPackageManager = read(
  "src/views/modules/environment/components/EnvPackageManagerDialog.vue",
);
const environmentPackageList = read(
  "src/views/modules/environment/components/EnvPackageList.vue",
);
const environmentLogDialog = read(
  "src/views/modules/environment/components/EnvLogDialog.vue",
);
const fileView = read("src/views/modules/File.vue");
const fileToolbar = read("src/views/modules/file/components/FileToolbar.vue");
const telegramCommands = read("src/views/modules/telegram/components/TelegramCommandsTab.vue");
const telegramWorkflows = read("src/views/modules/telegram/components/TelegramWorkflowsTab.vue");
const auditLog = read("src/views/modules/settings/AuditLogTab.vue");
const terminalSession = read("src/views/modules/terminal/composables/useTerminalSession.ts");
const overview = read("src/views/modules/overview/index.vue");
const shareView = read("src/views/modules/share/index.vue");
const pluginHost = read("src/views/plugins/PluginHostView.vue");
const extensionManager = read("src/views/modules/extensions/components/ExtensionManager.vue");
const extensionManagerState = read("src/views/modules/extensions/composables/useExtensionManager.ts");
const router = read("src/router/index.ts");

check(uno.includes("md: '769px'"), "UnoCSS md breakpoint must begin at 769px");
check(
  responsiveConstants.includes("MOBILE_MAX_WIDTH = 768"),
  "shared mobile breakpoint constant is missing",
);
check(
  appStore.includes("width.value <= MOBILE_MAX_WIDTH") &&
    appStore.includes("newWidth <= MOBILE_MAX_WIDTH"),
  "app store must use the shared mobile breakpoint",
);

check(!viewport.includes("user-scalable=no"), "viewport must allow user zoom");
check(!viewport.includes("maximum-scale"), "viewport must not cap zoom scale");
check(viewport.includes("viewport-fit=cover"), "viewport-fit=cover is required for safe areas");

for (const token of [
  "--app-viewport-height: 100vh",
  "--app-viewport-height: 100dvh",
  "--mobile-touch-target: 44px",
  "--mobile-header-height:",
  "--mobile-dock-clearance:",
  "@media (prefers-reduced-motion: reduce)",
]) {
  check(globalCss.includes(token), `global responsive token/rule missing: ${token}`);
}

for (const selector of [
  "button:not(.el-switch__core)",
  ".el-button:not(.is-link)",
  '[class*="text-[9px]"]',
  '[class*="text-[10px]"]',
]) {
  check(
    !globalCss.includes(selector),
    `mobile styles must not globally override compact controls: ${selector}`,
  );
}

check(
  !/@media\s*\(max-width:\s*768px\)[\s\S]*?\.el-popper\s*\{/.test(globalCss),
  "mobile styles must not globally constrain every Element Plus popper",
);

check(mainLayout.includes('class="app-viewport'), "main layout must use the dynamic app viewport");
check(mainLayout.includes("mobile-main-content"), "mobile main content must reserve Dock clearance");
check(header.includes("route.meta.title"), "mobile header must use route title metadata");
check(header.includes("@click=\"emit('open-search')\""), "mobile header search action is missing");

check(dock.includes("overflow-x-auto"), "mobile Dock must be horizontally scrollable");
check(dock.includes("scrollIntoView"), "mobile Dock must keep the active item visible");
check(dock.includes("pluginApps.mobileApps"), "mobile Dock must use mobile-filtered plugins");
check(
  pluginApps.includes("item.ui.display.mobile") && pluginApps.includes("const mobileApps"),
  "plugin store must expose apps allowed on mobile",
);

for (const contract of [
  'type ResponsiveDialogMobileMode = "sheet" | "fullscreen"',
  'export type ResponsiveDialogDesktopSize = "sm" | "md" | "lg" | "xl" | "fluid"',
  "contentPreset?: OverlayContentPreset",
  "desktopSize?: ResponsiveDialogDesktopSize",
  "<OverlayDrawer",
  "#header-actions",
]) {
  check(responsiveDialog.includes(contract), `ResponsiveDialog contract missing: ${contract}`);
}

for (const contract of [
  'export type OverlayContentPreset = "form" | "list" | "workspace"',
  'export type OverlayDrawerVariant = "sheet" | "side" | "workspace"',
  "var(--app-viewport-height)",
]) {
  check(overlayDrawer.includes(contract), `OverlayDrawer contract missing: ${contract}`);
}

check(
  overlayFooter.includes("env(safe-area-inset-bottom") &&
    overlayFooter.includes("var(--mobile-touch-target)"),
  "shared overlay footer must preserve the safe area and mobile touch size",
);

check(
  overlayHeader.includes("var(--mobile-touch-target)") &&
    overlayHeader.includes("focus-visible"),
  "shared overlay header must preserve mobile touch size and keyboard focus",
);

const directDialogs = sourceFiles
  .filter((path) => path.endsWith(".vue") && !path.endsWith("/ResponsiveDialog.vue"))
  .filter((path) => readFileSync(path, "utf8").includes("<el-dialog"));
check(
  directDialogs.length === 0,
  `direct el-dialog usage remains: ${directDialogs.map((path) => path.replace(`${root}/`, "")).join(", ")}`,
);

const directDrawers = sourceFiles
  .filter((path) => path.endsWith(".vue") && !path.endsWith("/OverlayDrawer.vue"))
  .filter((path) => readFileSync(path, "utf8").includes("<el-drawer"));
check(
  directDrawers.length === 0,
  `direct el-drawer usage remains: ${directDrawers.map((path) => path.replace(`${root}/`, "")).join(", ")}`,
);

const dynamicOverlayFiles = sourceFiles
  .filter((path) => path.endsWith(".vue"))
  .filter((path) =>
    /:is\s*=\s*["'][^"']*el-(?:dialog|drawer)/.test(
      readFileSync(path, "utf8"),
    ),
  );
check(
  dynamicOverlayFiles.length === 0,
  `dynamic dialog/drawer switching remains: ${dynamicOverlayFiles.map((path) => path.replace(`${root}/`, "")).join(", ")}`,
);

const unclassifiedResponsiveDialogs = sourceFiles
  .filter((path) => path.endsWith(".vue") && !path.endsWith("/ResponsiveDialog.vue"))
  .flatMap((path) => {
    const source = readFileSync(path, "utf8");
    return [...source.matchAll(/<ResponsiveDialog\b[\s\S]*?>/g)]
      .filter(
        ([tag]) =>
          !tag.includes("content-preset=") || !tag.includes("desktop-size="),
      )
      .map(() => path);
  });
check(
  unclassifiedResponsiveDialogs.length === 0,
  `ResponsiveDialog preset missing: ${[...new Set(unclassifiedResponsiveDialogs)].map((path) => path.replace(`${root}/`, "")).join(", ")}`,
);

const legacyOverlayClasses = [
  "modern-dialog",
  "log-modal",
  "action-sheet-drawer",
  "task-wizard-dialog",
  "cloud-dialog",
  "release-notes-dialog",
  "telegram-settings-drawer",
];
for (const className of legacyOverlayClasses) {
  const matches = sourceFiles.filter((path) =>
    readFileSync(path, "utf8").includes(className),
  );
  check(
    matches.length === 0,
    `legacy overlay class remains (${className}): ${matches.map((path) => path.replace(`${root}/`, "")).join(", ")}`,
  );
}

const leakedElementOverlayStyles = sourceFiles
  .filter(
    (path) =>
      path.endsWith(".vue") &&
      !path.endsWith("/ResponsiveDialog.vue") &&
      !path.endsWith("/OverlayDrawer.vue"),
  )
  .filter((path) =>
    /\.el-(?:dialog|drawer)__(?:header|body|footer)/.test(
      readFileSync(path, "utf8"),
    ),
  );
check(
  leakedElementOverlayStyles.length === 0,
  `business overlay shell overrides remain: ${leakedElementOverlayStyles.map((path) => path.replace(`${root}/`, "")).join(", ")}`,
);

const deprecatedInfiniteScrollFiles = sourceFiles
  .filter((path) => path.endsWith(".vue"))
  .filter((path) => readFileSync(path, "utf8").includes("v-infinite-scroll"));
check(
  deprecatedInfiniteScrollFiles.length === 0,
  `deprecated v-infinite-scroll usage remains: ${deprecatedInfiniteScrollFiles.map((path) => path.replace(`${root}/`, "")).join(", ")}`,
);

const deprecatedCheckboxLabelFiles = sourceFiles
  .filter((path) => path.endsWith(".vue"))
  .filter((path) =>
    /<el-checkbox(?=[\s>])(?:(?!>).)*\blabel\s*=/gs.test(
      readFileSync(path, "utf8"),
    ),
  );
check(
  deprecatedCheckboxLabelFiles.length === 0,
  `deprecated el-checkbox label-as-value usage remains: ${deprecatedCheckboxLabelFiles.map((path) => path.replace(`${root}/`, "")).join(", ")}`,
);

check(
  !/beforeEach\s*\(\s*async\s*\([^)]*\bnext\b/.test(router),
  "router guards must use return values instead of the deprecated next callback",
);

const desktopAt768 = sourceFiles.filter((path) =>
  /@media\s*\(min-width:\s*768px\)/.test(readFileSync(path, "utf8")),
);
check(
  desktopAt768.length === 0,
  `desktop media query still starts at 768px: ${desktopAt768.map((path) => path.replace(`${root}/`, "")).join(", ")}`,
);

const legacyViewportFiles = sourceFiles.filter((path) => {
  if (path.endsWith("/assets/styles/index.css")) return false;
  const source = readFileSync(path, "utf8");
  return /\b(?:h-screen|min-h-screen)\b|(?:height|max-height):\s*calc\(100vh|(?:height|max-height):\s*100vh/.test(source);
});
check(
  legacyViewportFiles.length === 0,
  `legacy mobile viewport sizing remains: ${legacyViewportFiles.map((path) => path.replace(`${root}/`, "")).join(", ")}`,
);

check(fileView.includes(':items="sortedFileList"'), "mobile Files list must use sorted items");
check(fileToolbar.includes("handleSortCommand"), "mobile Files toolbar must expose sorting");
check(fileToolbar.includes('placeholder="搜索文件"'), "mobile Files toolbar must keep search visible");
check(telegramCommands.includes('v-if="isMobile"'), "Telegram commands need a mobile card layout");
check(telegramWorkflows.includes('v-if="isMobile"'), "Telegram workflows need a mobile card layout");
check(!auditLog.includes("calc(100vh"), "audit log must use remaining flex height");
check(
  auditLog.includes("audit-mobile-card") &&
    auditLog.includes("overflow-x-hidden") &&
    auditLog.includes(':key="row.id"'),
  "audit log must keep its mobile card layout bounded and use stable row keys",
);
check(terminalSession.includes("new ResizeObserver"), "terminal must observe its container size");
check(terminalSession.includes("window.visualViewport"), "terminal must react to visual viewport changes");
check(
  overview.includes('query: { section: "audit" }'),
  "overview audit action must navigate to the settings audit section",
);
check(
  /<template>\s*<div class="h-full min-h-0">/.test(shareView),
  "share view must keep a single element root for RouterView transitions",
);
check(
  /<template>\s*<div class="h-full min-h-0">/.test(environmentView),
  "environment view must keep a single element root for RouterView transitions",
);
check(
  !environmentPackageManager.includes("手动卸载") &&
    !environmentPackageManager.includes("dependency-manager-overview") &&
    !environmentPackageManager.includes(
      "border-b border-[var(--editor-border)] px-5 py-4",
    ),
  "environment package manager must not restore the legacy manual-uninstall toolbar",
);
check(
  environmentPackageManager.includes("dependency-manager-toolbar") &&
    environmentPackageManager.includes("!min-h-11") &&
    environmentPackageList.includes('event: "uninstall"'),
  "environment package manager must keep its mobile toolbar and row-level uninstall action",
);
check(
  environmentLogDialog.includes("<ResponsiveDialog") &&
    environmentLogDialog.includes('mobile-mode="fullscreen"') &&
    !environmentLogDialog.includes("'el-drawer' : 'el-dialog'"),
  "environment logs must use the shared responsive dialog contract",
);
check(
  environmentLogDialog.includes("environment-log-shell") &&
    environmentLogDialog.includes("#header-actions") &&
    environmentLogDialog.includes("mobile-touch-target") &&
    environmentLogDialog.includes("compact"),
  "environment logs must keep a bounded viewer and mobile-sized toolbar action",
);
check(
  extensionManager.includes("<ExtensionImpactPreviewDialog") &&
    extensionManagerState.includes("impactDialog") &&
    extensionManagerState.includes("resolveImpactPreview") &&
    !extensionManagerState.includes("extension-impact-preview-dialog"),
  "complex extension impact previews must use the shared responsive dialog",
);
check(
  pluginHost.includes("该插件暂未适配移动端") && pluginHost.includes("app.ui.display.mobile"),
  "plugin host must explain unsupported mobile plugins",
);

if (failures.length > 0) {
  console.error(`Mobile responsive UI verification failed:\n- ${failures.join("\n- ")}`);
  process.exit(1);
}

console.log("Mobile responsive UI verification passed.");
