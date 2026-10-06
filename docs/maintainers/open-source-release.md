# Open-source release checklist

This repository previously contained local runtime state and downloaded build tools. A public release must be produced from a clean Git history, not only from a clean working tree.

## Required checks

1. Confirm `LICENSE`, `SECURITY.md`, `CONTRIBUTING.md`, and `CODE_OF_CONDUCT.md` are present.
2. Confirm the README describes the project as open source and documents the Apache-2.0 license.
3. Run `bash scripts/verify-public-release-gate.sh`.
4. Run `node scripts/verify-version-contract.mjs Core-v<core-version>` or `node scripts/verify-version-contract.mjs web-v<web-version>` and ensure the component Tag matches its manifest.
5. Inspect `git status` and `git ls-files` for runtime state, archives, private keys, and downloaded executables.
6. Run a secret scanner such as Gitleaks or TruffleHog against the complete history.
7. Generate release archives in CI; do not commit them to the source branch.

## History cleanup

Removing a secret or database in a new commit does not remove it from older commits. Before creating the public repository, use a new squashed root commit or `git filter-repo` to remove at least:

```text
data/
release_tools/
magisk/tools/
docker/*.tar.gz
target*/
```

Rotate any credentials, session keys, API keys, bot tokens, signing keys, or passwords that may ever have existed in those files. Treat historic values as compromised.

## Third-party tools

Release and Magisk packages bundle `uv` plus pnpm; they must never bundle fnm. `scripts/prepare-runtime-tools.sh` prepares pnpm from pinned `@pnpm/exe` artifacts and verifies SHA-512 before packaging. NiuPanel's pnpm 12.9.1 bundles target AMD64 and ARM64; upstream no longer supplies ARMv7 binaries, so new Core/Docker releases omit ARMv7. Legacy ARMv7 release descriptors remain readable and verified when present. NiuPanel retains the same verified first-use bootstrap as a fallback when the bundled executable cannot run on the host. Keep these pins and checksums current, and include upstream notices whenever a release bundle redistributes third-party binaries.

## Publishing model

- Source code: Apache License 2.0.
- Core Release 使用 `Core-vX.Y.Z`，Web Release 使用 `web-vX.Y.Z`；组件版本使用纯数字。Panel Release 使用 `vX.Y.Z` 或 `vX.Y.Z-beta.N`，预发布版本只能进入 `preview` 通道。Launcher 保持独立版本，通过 `RELEASE_PROTOCOL_VERSION` 与 Core 协商。
- Web 在 `release-manifest.json` 中声明兼容的 Core 范围；Core 与 Web 均从 0.8.0 起采用新更新协议，不兼容 0.7.x 的旧 Release 格式。
- Docker keeps an independent environment version in `docker/VERSION`; bump it only when the base image, system dependencies, bundled runtime tools, or container contract changes.
- **日常发布统一使用 `Release NiuPanel`**：在 Actions 选择发布代码所在分支，填写 Panel 版本和 `preview` / `stable` 通道即可。流水线读取该提交中的 Core、Web 版本，自动准备两个组件 Tag，并行构建和校验，随后发布 Panel 和更新通道。无需手动执行三个工作流。
- 统一发布固定同一个源码提交。每次新发布需要同时递增 `niupanel/Cargo.toml` 与 `niupanelweb/package.json` 的版本，并同步 `Cargo.lock`；例如本次 Panel `0.8.6` 对应 Core `0.8.6`、Web `2.0.6`。已存在的任何组件或 Panel Tag 若属于其他提交，预检会直接拒绝，不移动旧 Tag。
- 失败时在原 Actions 运行中选择 **Re-run failed jobs**。组件或 Panel 任一步未完成，都不会继续正常执行下一阶段；已公开资产继续受不可变校验保护。不要通过重新在更新后的分支上运行工作流来替换同版本产物。
- GitHub 的 Core/Web 预发布资产和旧 Tag 格式仍保留，以兼容旧客户端和已有下载地址；它们是统一发布的内部步骤，用户更新入口仍是一个 Panel 版本。Launcher、协议校验和回退机制不变。
- `Publish Core Pre-release` 与 `Publish Web Pre-release` 作为高级入口保留，只构建并发布不可变组件，不修改通道。
- 高级用法：`Publish Panel Release` 选择一个现有 `Core-v...` 与 `web-v...`，验证资产与兼容契约，把不可变组合描述附加到 Panel Release，然后原子更新一个通道指针。纯前端修复复用原 Core Tag。
- `Promote Panel Release` 可将使用纯数字版本的 preview Panel Release 原地提升为 stable，并把同一完整描述写入 `stable.json`；带 `-beta.N` 的预览版本不能改名，正式发布需创建新的纯数字 Panel Release。流程可安全重跑。
- `main/release/channels` 是面板更新和 Docker 构建的唯一输入，每个 schema v2 文件只指向一个完整 Panel Release。
- 本地 `./build.sh [amd64|arm64|all]` 仍可生成组合测试包；它不是线上通道索引或正式组件发布的来源。
- `Publish Docker Image` 仅手动运行，使用所选分支的 Dockerfile，通道索引始终从 main 读取。它验证所选通道后构建多架构镜像，只推送 `docker/VERSION`（当前 `3.0.3`）与 `latest`，不创建 preview 标签。Panel/Web 的常规在线更新不需要重建镜像。
- 新镜像内置的 Panel 版本只有在高于持久化 active 版本时才会由 Launcher 排队激活；因此重建同一 Docker 环境版本不会绕过 Panel Release 的不可变性或回退事务。
- 发布资产和索引写入使用 `RELEASE_TOKEN`（fine-grained PAT，目标仓库 `Contents: Read and write`、`Workflows: Read and write`）。统一入口仅使用 `GITHUB_TOKEN` 创建组件 Tag，避免再次触发独立的 Tag 发布流水线；Docker 镜像仍按需手动发布。
- Plugins: independently versioned packages; their manifests must declare their own license.
- User data and imported scripts: never part of the source distribution.

pnpm 12 迁移：开发镜像和运行时工具固定为 `12.9.1`。发布前使用 Docker 运行 `node --test scripts/update-components.test.mjs`，检查双架构产物及旧 ARMv7 索引的兼容性。旧客户端若强制要求索引包含三种架构，首次迁移须重建 Docker 容器或手动更新 Core，再使用新的双架构更新通道。

统一发布的本地预检（不创建 Tag、不访问 GitHub 发布 API）：

```sh
node scripts/verify-version-contract.mjs
PANEL_VERSION=0.8.6 UPDATE_CHANNEL=preview node scripts/prepare-panel-release.mjs --dry-run
node --test scripts/prepare-panel-release.test.mjs scripts/update-components.test.mjs
```
