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
];

const failures = checks
  .filter(([, passed]) => !passed)
  .map(([name]) => `- ${name}`);

if (failures.length > 0) {
  console.error(`File scroll UI verification failed:\n${failures.join("\n")}`);
  process.exit(1);
}

console.log(`File scroll UI verification passed (${checks.length} checks).`);
