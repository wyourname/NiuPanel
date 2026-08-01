#!/usr/bin/env node
import { existsSync } from "node:fs";
import { cp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";

const usage = `
Usage:
  node scripts/create-private-plugin-repo.mjs <target-dir> [options]

Options:
  --agents-id <id>       Agents app plugin id (default: private-agent-app)
  --compiler-id <id>     Compiler/loader plugin id (default: private-compiler-loader)
  --force                Remove target-dir before generating
  -h, --help             Show help
`.trim();

const args = process.argv.slice(2);
if (args.includes("-h") || args.includes("--help")) {
  console.log(usage);
  process.exit(0);
}

const targetArg = args.find((arg) => !arg.startsWith("--"));
if (!targetArg) fail(usage);

const options = parseOptions(args.filter((arg) => arg !== targetArg));
const repoRoot = process.cwd();
const targetDir = path.resolve(targetArg);
const agentsId = options.agentsId ?? "private-agent-app";
const compilerId = options.compilerId ?? "private-compiler-loader";

validatePluginId(agentsId, "agents id");
validatePluginId(compilerId, "compiler id");

if (existsSync(targetDir)) {
  if (!options.force) {
    fail(`Target already exists: ${targetDir}. Use --force to replace it.`);
  }
  await rm(targetDir, { recursive: true, force: true });
}

await mkdir(targetDir, { recursive: true });
await mkdir(path.join(targetDir, "scripts"), { recursive: true });
await mkdir(path.join(targetDir, "plugins"), { recursive: true });
await mkdir(path.join(targetDir, "packages"), { recursive: true });
await mkdir(path.join(targetDir, "schemas"), { recursive: true });

await cp(
  path.join(repoRoot, "scripts", "package-plugin.mjs"),
  path.join(targetDir, "scripts", "package-plugin.mjs"),
);
await cp(
  path.join(repoRoot, "scripts", "generate-plugin-signing-key.mjs"),
  path.join(targetDir, "scripts", "generate-plugin-signing-key.mjs"),
);
await cp(
  path.join(repoRoot, "docs", "plugins", "plugin.schema.json"),
  path.join(targetDir, "schemas", "plugin.schema.json"),
);
await cp(
  path.join(repoRoot, "packages", "plugin-sdk"),
  path.join(targetDir, "packages", "plugin-sdk"),
  { recursive: true, filter: excludeDevelopmentArtifacts },
);

const agentsTarget = path.join(targetDir, "plugins", agentsId);
const compilerTarget = path.join(targetDir, "plugins", compilerId);

await cp(path.join(repoRoot, "examples", "plugins", "agents", "app-template"), agentsTarget, {
  recursive: true,
  filter: excludeDevelopmentArtifacts,
});
await cp(path.join(repoRoot, "examples", "plugins", "compiler", "echo-compiler"), compilerTarget, {
  recursive: true,
  filter: excludeDevelopmentArtifacts,
});

await rewriteAgentsTemplate(agentsTarget, agentsId);
await rewriteCompilerTemplate(compilerTarget, compilerId);
await writePrivateRepoPackageJson(targetDir, agentsId, compilerId);
await writePrivateRepoReadme(targetDir, agentsId, compilerId);
await writeGitignore(targetDir);
await writeFile(
  path.join(targetDir, ".npmrc"),
  "registry=https://registry.npmmirror.com/\nprefer-offline=true\n",
);
await writeFile(
  path.join(targetDir, "pnpm-workspace.yaml"),
  "allowBuilds:\n  esbuild: true\n  vue-demi: true\nnodeVersion: 22.23.1\nengineStrict: true\n",
);

console.log(
  JSON.stringify(
    {
      ok: true,
      target: targetDir,
      agents_plugin: `plugins/${agentsId}`,
      compiler_plugin: `plugins/${compilerId}`,
    },
    null,
    2,
  ),
);

function parseOptions(values) {
  const parsed = {};
  for (let index = 0; index < values.length; index += 1) {
    const arg = values[index];
    if (arg === "--force") {
      parsed.force = true;
    } else if (arg === "--agents-id") {
      parsed.agentsId = requireValue(values, ++index, arg);
    } else if (arg === "--compiler-id") {
      parsed.compilerId = requireValue(values, ++index, arg);
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

async function rewriteAgentsTemplate(pluginDir, pluginId) {
  const manifestPath = path.join(pluginDir, "plugin.json");
  const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
  manifest.$schema = "../../schemas/plugin.schema.json";
  manifest.id = pluginId;
  manifest.name = "Private Agent App";
  manifest.description = "Private native Vue agents plugin app.";
  manifest.ui.display.category = "agents";
  manifest.ui.routes = manifest.ui.routes.map((route) => ({
    ...route,
    path: route.path.replace("/plugins/agent-app-template", `/plugins/${pluginId}`),
  }));
  await writeJson(manifestPath, manifest);

  await replaceInFile(
    path.join(pluginDir, "README.md"),
    [
      ["agent-app-template", pluginId],
      ["Agent App Plugin Template", "Private Agent App Plugin"],
      ["Agent App Template", "Private Agent App"],
      ["examples/plugins/agents/app-template", `plugins/${pluginId}`],
    ],
  );
  await replaceInFile(path.join(pluginDir, "ui", "package.json"), [
    ["agent-app-template-ui", `${pluginId}-ui`],
    ["../../../../../packages/plugin-sdk", "../../../packages/plugin-sdk"],
  ]);
  await replaceInFile(path.join(pluginDir, "ui", "vite.config.ts"), [
    ["../../../../../packages/plugin-sdk/src", "../../../packages/plugin-sdk/src"],
  ]);
  await replaceInFile(path.join(pluginDir, "ui", "src", "App.vue"), [
    ["/plugins/agent-app-template", `/plugins/${pluginId}`],
    ["Agent app template", "Private agent app"],
  ]);
}

async function rewriteCompilerTemplate(pluginDir, pluginId) {
  const manifestPath = path.join(pluginDir, "plugin.json");
  const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
  manifest.$schema = "../../schemas/plugin.schema.json";
  manifest.id = pluginId;
  manifest.name = "Private Compiler Loader";
  manifest.description = "Private compiler/loader plugin package.";
  if (Array.isArray(manifest.ui?.routes)) {
    manifest.ui.routes = manifest.ui.routes.map((route) => ({
      ...route,
      path: route.path.replace("/plugins/echo-compiler", `/plugins/${pluginId}`),
      title: route.title === "Echo Compiler" ? "Private Compiler Loader" : route.title,
    }));
  }
  await writeJson(manifestPath, manifest);

  await replaceInFile(path.join(pluginDir, "README.md"), [
    ["Echo Compiler Plugin", "Private Compiler Loader Plugin"],
    ["echo-compiler", pluginId],
    ["Echo Compiler", "Private Compiler Loader"],
    ["examples/plugins/compiler/echo-compiler", `plugins/${pluginId}`],
  ]);
  await replaceInFile(path.join(pluginDir, "main.js"), [
    ["echo-compiler example plugin", "private compiler loader plugin"],
  ]);
}

async function writePrivateRepoPackageJson(targetDir, agentsId, compilerId) {
  const json = {
    name: "niupanel-private-plugins",
    version: "0.1.0",
    private: true,
    type: "module",
    packageManager: "pnpm@11.18.0",
    engines: {
      node: ">=22.13.0",
    },
    scripts: {
      "build:agents-ui": `pnpm --dir plugins/${agentsId}/ui install && pnpm --dir plugins/${agentsId}/ui run build`,
      "package:agents": `pnpm run build:agents-ui && node scripts/package-plugin.mjs plugins/${agentsId} --out dist/plugins --market dist/plugins/index.json`,
      "package:compiler": `node scripts/package-plugin.mjs plugins/${compilerId} --out dist/plugins --market dist/plugins/index.json`,
      "package:all": "pnpm run package:agents && pnpm run package:compiler",
      "package:compiler:signed": `node scripts/package-plugin.mjs plugins/${compilerId} --out dist/plugins --market dist/plugins/index.json --sign-key "$PLUGIN_SIGN_KEY"`,
      "package:agents:signed": `pnpm run build:agents-ui && node scripts/package-plugin.mjs plugins/${agentsId} --out dist/plugins --market dist/plugins/index.json --sign-key "$PLUGIN_SIGN_KEY"`,
      "signing:keygen": "node scripts/generate-plugin-signing-key.mjs",
      "validate:compiler": `node scripts/package-plugin.mjs plugins/${compilerId} --dry-run`,
      "validate:agents": `pnpm run build:agents-ui && node scripts/package-plugin.mjs plugins/${agentsId} --dry-run`,
      verify: "pnpm run validate:compiler && pnpm run validate:agents",
    },
  };
  await writeJson(path.join(targetDir, "package.json"), json);
}

async function writePrivateRepoReadme(targetDir, agentsId, compilerId) {
  await writeFile(
    path.join(targetDir, "README.md"),
    `# NiuPanel Private Plugins

This repository is a private plugin workspace generated from the public
NiuPanel plugin contract. Keep private agents and loader/compiler code here,
then publish only packaged artifacts to your private plugin market.

## Layout

- \`plugins/${agentsId}\`: native Vue agents App.
- \`plugins/${compilerId}\`: compiler/loader App.
- \`packages/plugin-sdk\`: local copy of the NiuPanel plugin SDK used by Vue UI builds.
- \`schemas/plugin.schema.json\`: JSON Schema for \`plugin.json\` editing and review.
- \`scripts/package-plugin.mjs\`: validates, packages, hashes, and updates \`dist/plugins/index.json\`.
- \`scripts/generate-plugin-signing-key.mjs\`: creates an Ed25519 signing key and prints the trusted key pin.
- \`.pluginignore\`: optional per-plugin package ignore file for excluding build sources from \`.tgz\` artifacts.

## Build And Package

\`\`\`bash
pnpm run verify
pnpm run package:compiler
pnpm run package:agents
\`\`\`

The agents plugin command runs the Vue UI build before validating or packaging.
The generated market files are written to:

\`\`\`text
dist/plugins/
dist/plugins/index.json
\`\`\`

Serve \`dist/plugins/index.json\` and the generated \`.tgz\` files from a private
HTTP endpoint, then add that URL in the NiuPanel plugin market settings.

For signed private markets, set an Ed25519 private key path and use the signed
package commands:

\`\`\`bash
pnpm run signing:keygen
PLUGIN_SIGN_KEY=/secure/plugin-ed25519.pem pnpm run package:compiler:signed
PLUGIN_SIGN_KEY=/secure/plugin-ed25519.pem pnpm run package:agents:signed
\`\`\`

The package command prints \`trusted_key\`. Add that value to
\`TRUSTED_PLUGIN_PUBLIC_KEYS\` for public-key pinning.

## Panel Integration

1. Serve \`dist/plugins/\` behind an authenticated private HTTP endpoint.
2. Add the market index URL in the panel plugin settings.
3. Install and enable \`${agentsId}\` for the native agents app.
4. Install and enable \`${compilerId}\` for compiler/loader actions.
5. Open the product-level agents app from \`/plugins/agents\`.

The agents plugin also keeps its own declared route under
\`/plugins/${agentsId}\`. Use \`/plugins/agents\` for the stable panel entry and
\`/plugins/${agentsId}\` when you need to address this specific plugin app.

The panel keeps the public API surface in the main repository and loads private
behavior from these packages. Do not copy private implementation files back into
the public panel repository.

## Migrating Private Code

- Put private agents UI and orchestration code under
  \`plugins/${agentsId}\`.
- Keep the agents route paths under \`/plugins/${agentsId}\`.
- Put private compiler/loader code under \`plugins/${compilerId}\`.
- Keep compiler runtime capabilities explicit:
  \`compiler.versions\` and \`compiler.encrypt\`.
- For closed-source binary plugins, build runtime artifacts into \`bin/\` and
  add backend source directories to \`.pluginignore\`.
- Re-run \`pnpm run verify\` before publishing a new market index.

## Public Boundary

The public panel repository only needs the plugin runtime, SDK contract, and
market installer. Private implementation code should stay in this repository and
ship as \`plugin.json\` packages.
`,
  );
}

async function writeGitignore(targetDir) {
  await writeFile(
    path.join(targetDir, ".gitignore"),
    `node_modules/
dist/
.DS_Store
target/
keys/*.pem
`,
  );
}

async function replaceInFile(file, replacements) {
  let content = await readFile(file, "utf8");
  for (const [from, to] of replacements) {
    content = content.split(from).join(to);
  }
  await writeFile(file, content);
}

async function writeJson(file, value) {
  await writeFile(file, `${JSON.stringify(value, null, 2)}\n`);
}

function excludeDevelopmentArtifacts(source) {
  const parts = source.split(path.sep);
  return !parts.some((part) =>
    [".git", "node_modules", "target", ".DS_Store"].includes(part),
  );
}

function validatePluginId(value, label) {
  if (!/^[a-z0-9][a-z0-9._-]{0,79}$/.test(value)) {
    fail(`${label} must use lowercase letters, digits, '.', '_' or '-'`);
  }
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
