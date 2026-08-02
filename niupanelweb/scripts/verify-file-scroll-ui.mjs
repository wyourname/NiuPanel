import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const fileView = readFileSync(resolve(root, "src/views/modules/File.vue"), "utf8");
const desktopList = readFileSync(
  resolve(root, "src/views/modules/file/components/FileDesktopList.vue"),
  "utf8",
);
const mobileList = readFileSync(
  resolve(root, "src/views/modules/file/components/FileMobileList.vue"),
  "utf8",
);
const editorDialog = readFileSync(
  resolve(root, "src/views/modules/file/components/FileEditorDialog.vue"),
  "utf8",
);
const responsiveDialog = readFileSync(
  resolve(root, "src/components/common/ResponsiveDialog.vue"),
  "utf8",
);
const toolbar = readFileSync(
  resolve(root, "src/views/modules/file/components/FileToolbar.vue"),
  "utf8",
);
const fileSort = readFileSync(
  resolve(root, "src/views/modules/file/utils/fileSort.ts"),
  "utf8",
);
const fileCodeEditor = readFileSync(
  resolve(root, "src/views/modules/file/components/FileCodeEditor.vue"),
  "utf8",
);
const fileWorkspaceEditor = readFileSync(
  resolve(root, "src/components/workspace/FileEditorWorkspaceWindow.vue"),
  "utf8",
);
const workspaceLayer = readFileSync(
  resolve(root, "src/components/workspace/WorkspaceLayer.vue"),
  "utf8",
);
const workspaceWindow = readFileSync(
  resolve(root, "src/components/workspace/WorkspaceWindow.vue"),
  "utf8",
);
const workspaceStore = readFileSync(
  resolve(root, "src/stores/workspace.ts"),
  "utf8",
);

const checks = [
  [
    "desktop list parent preserves a flex height chain",
    /<section\s+class="[^"]*\bflex\b[^"]*\bmin-h-0\b[^"]*\bflex-1\b[^"]*\bflex-col\b[^"]*\boverflow-hidden\b[^"]*"/.test(
      fileView,
    ),
  ],
  [
    "desktop list root has a bounded full-height container",
    /class="[^"]*\bh-full\b[^"]*\bmin-h-0\b[^"]*\bflex-1\b[^"]*\boverflow-hidden\b[^"]*"/.test(
      desktopList,
    ),
  ],
  [
    "desktop file list has the wheel-scrollable overflow container",
    /class="[^"]*\bh-full\b[^"]*\bmin-h-0\b[^"]*\boverflow-auto\b[^"]*\bcustom-scrollbar\b[^"]*"/.test(
      desktopList,
    ),
  ],
  [
    "mobile list root preserves a bounded flex height chain",
    /class="[^"]*\bmin-h-0\b[^"]*\bflex-1\b[^"]*\boverflow-hidden\b[^"]*"/.test(
      mobileList,
    ),
  ],
  [
    "mobile file list has its own vertical scroll container",
    /class="[^"]*\bh-full\b[^"]*\bmin-h-0\b[^"]*\boverflow-y-auto\b[^"]*\bcustom-scrollbar\b[^"]*"/.test(
      mobileList,
    ),
  ],
  [
    "file editor declares a bounded desktop workspace height",
    editorDialog.includes('desktop-height="min(760px, calc(var(--app-viewport-height) - 48px))"') &&
      editorDialog.includes('class="flex h-full min-h-0 flex-1 flex-col'),
  ],
  [
    "responsive dialog supports explicit desktop workspace height",
    responsiveDialog.includes("desktopHeight?: string | number") &&
      responsiveDialog.includes(':style="desktopDialogStyle"'),
  ],
  [
    "file sorting exposes newest-first and oldest-first modified-time choices",
    toolbar.includes("FILE_SORT_OPTIONS") &&
      fileSort.includes('value: "mtime-desc"') &&
      fileSort.includes('value: "mtime-asc"') &&
      fileSort.includes("compareModifiedTime"),
  ],
  [
    "desktop files open in the movable and resizable workspace window",
    fileView.includes("workspaceStore.openFileEditorWindow(item)") &&
      workspaceStore.includes('openWindow("file-editor"') &&
      workspaceLayer.includes("<FileEditorWorkspaceWindow"),
  ],
  [
    "desktop and mobile file editors share one automatically resizing Monaco surface",
    editorDialog.includes("<FileCodeEditor") &&
      fileWorkspaceEditor.includes("<FileCodeEditor") &&
      fileCodeEditor.includes("automaticLayout: true"),
  ],
  [
    "file editor protects unsaved workspace content before closing",
    fileWorkspaceEditor.includes("registerCloseGuard") &&
      workspaceStore.includes("requestCloseWindow"),
  ],
  [
    "workspace windows remain reachable after dragging, resizing, and viewport changes",
    workspaceWindow.includes("fitWindowToViewport") &&
      workspaceWindow.includes("maxWidth") &&
      workspaceWindow.includes("maxHeight"),
  ],
];

const failures = checks
  .filter(([, passed]) => !passed)
  .map(([name]) => `- ${name}`);

if (failures.length > 0) {
  console.error(`File scroll UI verification failed:\n${failures.join("\n")}`);
  process.exit(1);
}

console.log(`File scroll UI verification passed (${checks.length} checks).`);
