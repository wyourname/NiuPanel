#!/usr/bin/env node
import { createHash, createPrivateKey, createPublicKey, sign } from "node:crypto";
import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import {
  cp,
  mkdir,
  mkdtemp,
  readFile,
  rename,
  rm,
  stat,
  writeFile,
} from "node:fs/promises";
import os from "node:os";
import path from "node:path";

const usage = `
Usage:
  node scripts/package-plugin.mjs <plugin-dir> [options]

Options:
  --out <dir>           Output directory for <plugin-id>.tgz (default: dist/plugins)
  --market <file>       Create or update a plugin market index JSON file
  --download-url <url>  download_url written to market index (default: ./<plugin-id>.tgz)
  --sign-key <pem>      Ed25519 private key PEM used to sign the package bytes
  --build-ui            Run pnpm run build in ./ui before validation when ui/package.json exists
  --index-name <name>   Market index name when creating a new file
  --dry-run             Validate only; do not write package or market index
  -h, --help            Show help
`.trim();

const args = process.argv.slice(2);
if (args.includes("-h") || args.includes("--help")) {
  console.log(usage);
  process.exit(0);
}

const pluginDirArg = args.find((arg) => !arg.startsWith("--"));
if (!pluginDirArg) fail(usage);

const options = parseOptions(args.filter((arg) => arg !== pluginDirArg));
const pluginDir = path.resolve(pluginDirArg);
const manifestPath = path.join(pluginDir, "plugin.json");
const manifest = await readJson(manifestPath, "plugin.json");
const packageIgnore = await readPackageIgnore(pluginDir);

if (options.buildUi) {
  await buildUi(pluginDir);
}
await validateManifest(pluginDir, manifest);

const fileName = `${manifest.id}.tgz`;
const outDir = path.resolve(options.out ?? "dist/plugins");
const packagePath = path.join(outDir, fileName);
const downloadUrl = options.downloadUrl ?? `./${fileName}`;

if (options.dryRun) {
  console.log(
    JSON.stringify(
      {
        ok: true,
        dry_run: true,
        id: manifest.id,
        version: manifest.version,
        capabilities: manifest.capabilities,
      },
      null,
      2,
    ),
  );
  process.exit(0);
}

await mkdir(outDir, { recursive: true });
await createPackage(pluginDir, manifest.id, packagePath, packageIgnore);
const checksum = await sha256File(packagePath);
const signature = options.signKey ? await signPackage(packagePath, options.signKey) : null;

let marketPath = null;
if (options.market) {
  marketPath = path.resolve(options.market);
  await updateMarketIndex(marketPath, manifest, downloadUrl, checksum, signature, options);
}

console.log(
  JSON.stringify(
    {
      ok: true,
      id: manifest.id,
      version: manifest.version,
      capabilities: manifest.capabilities,
      package: packagePath,
      checksum_sha256: checksum,
      signature_ed25519: signature?.signature_ed25519 ?? null,
      public_key_ed25519: signature?.public_key_ed25519 ?? null,
      trusted_key: signature?.trusted_key ?? null,
      market: marketPath,
    },
    null,
    2,
  ),
);

function parseOptions(values) {
  const parsed = {};
  for (let index = 0; index < values.length; index += 1) {
    const arg = values[index];
    if (arg === "--build-ui") {
      parsed.buildUi = true;
    } else if (arg === "--dry-run") {
      parsed.dryRun = true;
    } else if (arg === "--out") {
      parsed.out = requireValue(values, ++index, arg);
    } else if (arg === "--market") {
      parsed.market = requireValue(values, ++index, arg);
    } else if (arg === "--download-url") {
      parsed.downloadUrl = requireValue(values, ++index, arg);
    } else if (arg === "--sign-key") {
      parsed.signKey = requireValue(values, ++index, arg);
    } else if (arg === "--index-name") {
      parsed.indexName = requireValue(values, ++index, arg);
    } else {
      fail(`Unknown option: ${arg}\n\n${usage}`);
    }
  }
  return parsed;
}

function requireValue(values, index, option) {
  const value = values[index];
  if (!value || value.startsWith("--")) fail(`${option} requires a value`);
  return value;
}

async function readJson(file, label) {
  try {
    return JSON.parse(await readFile(file, "utf8"));
  } catch (error) {
    fail(`Failed to read ${label}: ${error.message}`);
  }
}

async function readPackageIgnore(root) {
  const file = path.join(root, ".pluginignore");
  if (!existsSync(file)) return [];
  const content = await readFile(file, "utf8");
  return content
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith("#"))
    .map((line) => line.replace(/\\/g, "/").replace(/^\/+/, ""));
}

async function validateManifest(root, manifest) {
  requireNumber(manifest.schema_version, "schema_version");
  if (manifest.schema_version !== 1) fail("schema_version must be 1");
  requireIdentifier(manifest.id, "id");
  requireText(manifest.name, "name");
  requireText(manifest.version, "version");
  requireText(manifest.description, "description");
  requireText(manifest.runtime, "runtime");
  if (!["builtin", "declarative", "process", "wasi", "native"].includes(manifest.runtime)) {
    fail(`runtime is unsupported: ${manifest.runtime}`);
  }
  if (!["builtin", "declarative"].includes(manifest.runtime)) {
    requireText(manifest.entry, "entry");
    await requireExistingPath(root, manifest.entry, "entry");
  }
  validateEnvironment(manifest.env ?? {});
  validateRuntimePermissions(manifest.runtime_permissions ?? []);
  if (!Array.isArray(manifest.capabilities)) fail("capabilities must be an array");
  for (const capability of manifest.capabilities) validateCapability(capability);
  if (manifest.ui?.enabled) await validateUi(root, manifest.ui);
  if (manifest.theme?.enabled) validateTheme(manifest.theme);
}

function validateRuntimePermissions(permissions) {
  if (!Array.isArray(permissions)) fail("runtime_permissions must be an array");
  const allowed = new Set(["network_outbound"]);
  const seen = new Set();
  for (const permission of permissions) {
    if (!allowed.has(permission)) fail(`unsupported runtime permission: ${permission}`);
    if (seen.has(permission)) fail(`duplicate runtime permission: ${permission}`);
    seen.add(permission);
  }
}

function validateEnvironment(env) {
  for (const key of Object.keys(env)) {
    const normalized = key.toUpperCase();
    const reserved = normalized.startsWith("NIUPANEL_")
      || normalized.startsWith("LD_")
      || normalized.startsWith("DYLD_")
      || ["DATABASE_URL", "HOME", "PATH", "TMPDIR", "BASH_ENV", "ENV", "SHELLOPTS"].includes(normalized);
    if (!/^[A-Za-z_][A-Za-z0-9_]{0,127}$/.test(key) || reserved) {
      fail(`reserved or invalid plugin environment variable: ${key}`);
    }
  }
}

async function validateUi(root, ui) {
  if (ui.mode !== undefined && ui.mode !== "vue_app") fail("ui.mode must be vue_app");
  requireText(ui.entry, "ui.entry");
  await requireExistingFile(root, ui.entry, "ui.entry");
  if (!Array.isArray(ui.routes) || ui.routes.length === 0) {
    fail("ui.routes must be a non-empty array when ui.enabled is true");
  }
  const seen = new Set();
  for (const route of ui.routes) {
    requireText(route.path, "ui.routes[].path");
    if (!route.path.startsWith("/plugins/")) {
      fail(`ui route must start with /plugins/: ${route.path}`);
    }
    if (seen.has(route.path)) fail(`duplicate ui route path: ${route.path}`);
    seen.add(route.path);
    requireText(route.title, "ui.routes[].title");
  }
  if (!Array.isArray(ui.permissions)) fail("ui.permissions must be an array");
  const permissions = new Set();
  for (const permission of ui.permissions) {
    requireText(permission, "ui.permissions[]");
    if (permission.includes("*")) fail(`ui permission must be exact: ${permission}`);
    if (!/^[a-z]+:[a-z]+$/.test(permission)) fail(`invalid ui permission: ${permission}`);
    if (permissions.has(permission)) fail(`duplicate ui permission: ${permission}`);
    permissions.add(permission);
  }
  if (ui.api?.allow !== undefined && !Array.isArray(ui.api.allow)) {
    fail("ui.api.allow must be an array");
  }
  for (const rule of ui.api?.allow ?? []) validateApiRule(rule);
}

function validateApiRule(rule) {
  requireText(rule.path, "ui.api.allow[].path");
  if (!rule.path.startsWith("/") || rule.path === "/**") {
    fail(`ui api path must be an explicit panel path: ${rule.path}`);
  }
  if (!Array.isArray(rule.methods) || rule.methods.length === 0) {
    fail(`ui api methods must be explicit for ${rule.path}`);
  }
  for (const method of rule.methods) {
    if (!["GET", "POST", "PUT", "PATCH", "DELETE"].includes(method)) {
      fail(`unsupported ui api method: ${method}`);
    }
  }
}

function validateTheme(theme) {
  const palettes = [theme.light ?? {}, theme.dark ?? {}];
  const allowed = new Set([
    "primary",
    "bg_base",
    "bg_card",
    "bg_subtle",
    "bg_soft",
    "text_default",
    "text_secondary",
    "text_muted",
    "border_base",
    "border_light",
  ]);
  let tokenCount = 0;
  for (const palette of palettes) {
    for (const [key, value] of Object.entries(palette)) {
      if (!allowed.has(key)) fail(`unsupported theme token: ${key}`);
      if (!/^#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$/.test(value)) {
        fail(`theme token ${key} must be a 6 or 8 digit hex color`);
      }
      tokenCount += 1;
    }
  }
  if (tokenCount === 0) fail("enabled theme must define at least one color token");
}

async function requireExistingPath(root, relativePath, field) {
  const target = safeJoin(root, relativePath, field);
  try {
    await stat(target);
  } catch {
    fail(`${field} does not exist: ${relativePath}`);
  }
}

async function requireExistingFile(root, relativePath, field) {
  const target = safeJoin(root, relativePath, field);
  try {
    const info = await stat(target);
    if (!info.isFile()) fail(`${field} must be a file: ${relativePath}`);
  } catch {
    fail(`${field} does not exist: ${relativePath}`);
  }
}

function safeJoin(root, relativePath, field) {
  if (path.isAbsolute(relativePath)) fail(`${field} must be relative`);
  const target = path.resolve(root, relativePath);
  if (!target.startsWith(`${path.resolve(root)}${path.sep}`) && target !== path.resolve(root)) {
    fail(`${field} escapes plugin directory: ${relativePath}`);
  }
  return target;
}

function requireText(value, field) {
  if (typeof value !== "string" || value.trim() === "") fail(`${field} is required`);
}

function requireNumber(value, field) {
  if (typeof value !== "number" || !Number.isFinite(value)) fail(`${field} must be a number`);
}

function requireIdentifier(value, field) {
  requireText(value, field);
  if (!/^[a-z0-9][a-z0-9._-]{0,79}$/.test(value)) {
    fail(`${field} must use lowercase letters, digits, '.', '_' or '-'`);
  }
}

function validateCapability(value) {
  if (typeof value !== "string") fail("capabilities[] must be a string");
  const parts = value.trim().split(".");
  const valid =
    value.length <= 96 &&
    parts.length >= 2 &&
    parts.every((part, index) => {
      if (!part) return false;
      if (part === "*") return index === parts.length - 1;
      return /^[a-z0-9_-]+$/.test(part);
    });
  if (!valid) fail(`capability is invalid: ${value}`);
}

async function buildUi(root) {
  const uiDir = path.join(root, "ui");
  if (!existsSync(path.join(uiDir, "package.json"))) return;
  execFileSync("pnpm", ["run", "build"], {
    cwd: uiDir,
    stdio: "inherit",
  });
}

async function createPackage(sourceRoot, pluginId, packagePath, packageIgnore) {
  const tempRoot = await mkdtemp(path.join(os.tmpdir(), "niupanel-plugin-"));
  const stageRoot = path.join(tempRoot, pluginId);
  try {
    await cp(sourceRoot, stageRoot, {
      recursive: true,
      filter: (source) => shouldInclude(sourceRoot, source, packageIgnore),
    });
    execFileSync("tar", ["-czf", packagePath, "-C", tempRoot, pluginId], {
      stdio: "inherit",
    });
  } finally {
    await rm(tempRoot, { recursive: true, force: true });
  }
}

function shouldInclude(sourceRoot, source, packageIgnore) {
  const relative = path.relative(sourceRoot, source);
  if (!relative) return true;
  const normalized = relative.split(path.sep).join("/");
  if (isPackageIgnored(normalized, packageIgnore)) return false;
  const parts = normalized.split("/");
  return !parts.some((part) =>
    [
      ".git",
      ".idea",
      ".vscode",
      "node_modules",
      "target",
      ".DS_Store",
      ".pluginignore",
      "__pycache__",
    ].includes(part),
  );
}

function isPackageIgnored(relativePath, patterns) {
  return patterns.some((pattern) => {
    if (pattern.endsWith("/")) {
      const directory = pattern.replace(/\/+$/, "");
      return relativePath === directory || relativePath.startsWith(`${directory}/`);
    }
    if (pattern.includes("*")) {
      const escaped = pattern
        .split("*")
        .map((part) => part.replace(/[|\\{}()[\]^$+?.]/g, "\\$&"))
        .join(".*");
      return new RegExp(`^${escaped}$`).test(relativePath);
    }
    return relativePath === pattern || relativePath.startsWith(`${pattern}/`);
  });
}

async function sha256File(file) {
  const hash = createHash("sha256");
  hash.update(await readFile(file));
  return hash.digest("hex");
}

async function signPackage(packagePath, privateKeyPath) {
  const privateKeyPem = await readFile(path.resolve(privateKeyPath));
  const privateKey = createPrivateKey(privateKeyPem);
  if (privateKey.asymmetricKeyType !== "ed25519") {
    fail("--sign-key must be an Ed25519 private key");
  }
  const packageBytes = await readFile(packagePath);
  const signature = sign(null, packageBytes, privateKey);
  const publicKey = createPublicKey(privateKey);
  const publicPem = publicKey.export({ format: "pem", type: "spki" }).toString();
  const publicDer = publicKey.export({ format: "der", type: "spki" });
  const rawPublicKey = Buffer.from(publicDer).subarray(-32);
  const trustedKey = `sha256:${createHash("sha256").update(rawPublicKey).digest("hex")}`;
  return {
    signature_ed25519: signature.toString("base64"),
    public_key_ed25519: publicPem.trim(),
    trusted_key: trustedKey,
  };
}

async function updateMarketIndex(
  file,
  manifest,
  downloadUrl,
  checksum,
  signature,
  options,
) {
  let index;
  if (existsSync(file)) {
    index = await readJson(file, "market index");
  } else {
    index = {
      schema_version: 1,
      name: options.indexName ?? "NiuPanel Private Plugin Market",
      description: "Private plugin market index.",
      plugins: [],
    };
  }
  if (index.schema_version !== 1) fail("market index schema_version must be 1");
  if (!Array.isArray(index.plugins)) fail("market index plugins must be an array");

  const entry = {
    id: manifest.id,
    name: manifest.name,
    version: manifest.version,
    description: manifest.description,
    download_url: downloadUrl,
    checksum_sha256: checksum,
    signature_ed25519: signature?.signature_ed25519 ?? null,
    public_key_ed25519: signature?.public_key_ed25519 ?? null,
    permissions: manifest.ui?.permissions ?? [],
    homepage: null,
    repository: null,
  };

  const existing = index.plugins.findIndex((plugin) => plugin.id === manifest.id);
  if (existing >= 0) {
    index.plugins[existing] = { ...index.plugins[existing], ...entry };
  } else {
    index.plugins.push(entry);
  }
  index.plugins.sort((a, b) => a.id.localeCompare(b.id));

  await mkdir(path.dirname(file), { recursive: true });
  const temp = `${file}.${process.pid}.tmp`;
  await writeFile(temp, `${JSON.stringify(index, null, 2)}\n`);
  await rename(temp, file);
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
