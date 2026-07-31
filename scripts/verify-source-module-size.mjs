import { readFileSync, readdirSync, statSync } from "node:fs";
import { extname, join, relative } from "node:path";

const root = process.cwd();
const defaultMaxLines = 1200;
const architectureMaxLines = 500;
const sourceRoots = [
  "migration/src",
  "niupanel/src",
  "niupanel-bot/src",
  "niupanel-common/src",
  "niupanel-core/src",
  "niupanel-entity/src",
  "niupanel-launcher/src",
  "niupanel-plugin/src",
  "niupanel-proxy/src",
  "niupanel-sdk/src",
  "niupanelweb/src",
  "packages/plugin-sdk/src",
];
const architectureRoots = [
  "niupanel-plugin/src",
  "niupanel-launcher/src",
  "niupanel/src/modules/mcp",
  "niupanel/src/modules/plugins",
  "niupanel/src/modules/system",
  "niupanelweb/src/views/modules/extensions",
  "niupanelweb/src/views/plugins",
];
const sourceExtensions = new Set([".rs", ".ts", ".vue", ".js", ".mjs"]);

function sourceFiles(path) {
  const absolute = join(root, path);
  if (statSync(absolute).isFile()) return [absolute];
  return readdirSync(absolute, { withFileTypes: true }).flatMap((entry) => {
    const child = join(absolute, entry.name);
    return entry.isDirectory() ? sourceFiles(relative(root, child)) : [child];
  });
}

const oversized = sourceRoots
  .flatMap(sourceFiles)
  .filter((file) => sourceExtensions.has(extname(file)))
  .map((file) => ({
    file: relative(root, file),
    lines: readFileSync(file, "utf8").split(/\r?\n/).length,
  }))
  .map((entry) => ({
    ...entry,
    limit: architectureRoots.some(
      (path) => entry.file === path || entry.file.startsWith(`${path}/`),
    )
      ? architectureMaxLines
      : defaultMaxLines,
  }))
  .filter((entry) => entry.lines > entry.limit)
  .sort((left, right) => right.lines - left.lines);

if (oversized.length) {
  console.error("Source module size verification failed:");
  for (const entry of oversized) {
    console.error(`- ${entry.file}: ${entry.lines} lines (limit ${entry.limit})`);
  }
  process.exit(1);
}

console.log(
  `Source module size verified: all modules <= ${defaultMaxLines} lines; architecture modules <= ${architectureMaxLines} lines.`,
);
