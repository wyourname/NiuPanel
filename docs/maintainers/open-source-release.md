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

Release and Magisk packages bundle `uv` plus pnpm; they must never bundle fnm. `scripts/prepare-runtime-tools.sh` prepares pnpm from pinned `@pnpm/exe` artifacts and verifies SHA-512 before packaging. ARMv7 uses an additional pinned Node.js bootstrap verified by SHA-256. NiuPanel retains the same verified first-use bootstrap as a fallback when the bundled executable cannot run on the host. Keep these pins and checksums current, and include upstream notices whenever a release bundle redistributes third-party binaries.

## Publishing model

- Source code: Apache License 2.0.
- Core Release 使用 `Core-vX.Y.Z`，Web Release 使用 `web-vX.Y.Z`；组件版本使用纯数字。Panel Release 使用 `vX.Y.Z` 或 `vX.Y.Z-beta.N`，预发布版本只能进入 `preview` 通道。Launcher 保持独立版本，通过 `RELEASE_PROTOCOL_VERSION` 与 Core 协商。
- Web 在 `release-manifest.json` 中声明兼容的 Core 范围；Core 与 Web 均从 0.8.0 起采用新更新协议，不兼容 0.7.x 的旧 Release 格式。
- Docker keeps an independent environment version in `docker/VERSION`; bump it only when the base image, system dependencies, bundled runtime tools, or container contract changes.
- `Publish Core Pre-release` 与 `Publish Web Pre-release` 只构建并发布不可变组件，不修改通道。
- `Publish Panel Release` 选择一个现有 `Core-v...` 与 `web-v...`，验证资产与兼容契约，把不可变组合描述附加到 Panel Release，然后原子更新一个通道指针。纯前端修复复用原 Core Tag。
- `Promote Panel Release` 可将使用纯数字版本的 preview Panel Release 原地提升为 stable，并把同一完整描述写入 `stable.json`；带 `-beta.N` 的预览版本不能改名，正式发布需创建新的纯数字 Panel Release。流程可安全重跑。
- `main/release/channels` 是面板更新和 Docker 构建的唯一输入，每个 schema v2 文件只指向一个完整 Panel Release。
- 本地 `./build.sh [amd64|arm64|armv7|all]` 仍可生成组合测试包；它不是线上通道索引或正式组件发布的来源。
- `Publish Docker Image` 仅手动运行。它验证所选通道后构建多架构镜像，只推送 `docker/VERSION`（当前 `3.0.2`）与 `latest`，不创建 preview 标签。Panel/Web 的常规在线更新不需要重建镜像。
- 新镜像内置的 Panel 版本只有在高于持久化 active 版本时才会由 Launcher 排队激活；因此重建同一 Docker 环境版本不会绕过 Panel Release 的不可变性或回退事务。
- 发布和索引写入必须使用 `RELEASE_TOKEN`（fine-grained PAT，目标仓库 `Contents: Read and write`、`Workflows: Read and write`），以便写入 `main` 后可靠触发 Docker 工作流。
- Plugins: independently versioned packages; their manifests must declare their own license.
- User data and imported scripts: never part of the source distribution.
