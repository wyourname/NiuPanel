import { existsSync, readFileSync } from "node:fs";
import { execFileSync } from "node:child_process";

const read = (path) => readFileSync(path, "utf8");
const fail = (message) => {
  console.error(`Open-source readiness check failed: ${message}`);
  process.exit(1);
};

for (const file of [
  "LICENSE",
  "README.md",
  "CONTRIBUTING.md",
  "SECURITY.md",
  "CODE_OF_CONDUCT.md",
  ".env.example",
  "docs/architecture/repository-layout.md",
  "docs/maintainers/open-source-release.md",
]) {
  if (!existsSync(file)) fail(`missing ${file}`);
}

if (!read("LICENSE").includes("Apache License") || !read("LICENSE").includes("Version 2.0")) {
  fail("LICENSE is not the Apache License 2.0 text");
}

const readme = read("README.md");
if (!readme.includes("Apache License 2.0")) fail("README does not declare Apache-2.0");
if (/非开源|闭源分发|禁止.*商业用途/.test(readme)) {
  fail("README still contains closed-source or non-commercial-only language");
}

const tracked = execFileSync("git", ["ls-files", "-z"], { encoding: "utf8" })
  .split("\0")
  .filter(Boolean);
const forbiddenTracked = tracked.filter((path) =>
  /^(data|release_tools|magisk\/tools|target(?:_[^/]+)?|niupanelweb\/dist)\//.test(path)
  || /^docker\/.*\.tar\.gz$/.test(path)
  || /(^|\/)\.env(?:\.|$)/.test(path) && path !== ".env.example"
  || /\.(?:pem|key)$/.test(path),
);
if (forbiddenTracked.length) {
  fail(`runtime, generated, or secret files are tracked:\n${forbiddenTracked.join("\n")}`);
}

const rootCargo = read("Cargo.toml");
for (const metadata of [
  'license = "Apache-2.0"',
  'repository = "https://github.com/wyourname/NiuPanel.git"',
  'rust-version = "1.88"',
]) {
  if (!rootCargo.includes(metadata)) fail(`workspace package metadata is missing: ${metadata}`);
}

for (const manifest of tracked.filter(
  (path) => existsSync(path) && /^(?:niupanel[^/]*|migration)\/Cargo\.toml$/.test(path),
)) {
  const source = read(manifest);
  if (!source.includes("license.workspace = true")) {
    fail(`${manifest} does not inherit the workspace license`);
  }
}

for (const packageFile of ["niupanelweb/package.json", "packages/plugin-sdk/package.json"]) {
  const packageJson = JSON.parse(read(packageFile));
  if (packageJson.license !== "Apache-2.0") fail(`${packageFile} does not declare Apache-2.0`);
}

for (const manifest of tracked.filter(
  (path) => existsSync(path) && path.startsWith("examples/") && path.endsWith("/plugin.json"),
)) {
  const plugin = JSON.parse(read(manifest));
  if (!plugin.license) fail(`${manifest} does not declare an SPDX license`);
}

const config = read("niupanel-common/src/config.rs");
if (!config.includes("nanoid::nanoid!(64)")) {
  fail("SESSION_KEY fallback must be generated per installation/process");
}

console.log(`Open-source readiness verified across ${tracked.length} tracked files.`);
