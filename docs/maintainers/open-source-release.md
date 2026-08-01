# Open-source release checklist

This repository previously contained local runtime state and downloaded build tools. A public release must be produced from a clean Git history, not only from a clean working tree.

## Required checks

1. Confirm `LICENSE`, `SECURITY.md`, `CONTRIBUTING.md`, and `CODE_OF_CONDUCT.md` are present.
2. Confirm the README describes the project as open source and documents the Apache-2.0 license.
3. Run `bash scripts/verify-public-release-gate.sh`.
4. Run `node scripts/verify-version-contract.mjs core-v<core-version>` or `node scripts/verify-version-contract.mjs web-v<web-version>` and ensure the component Tag matches its manifest.
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
- Core Release 使用 `core-vX.Y.Z`，Web Release 使用 `web-vX.Y.Z`；版本号均为纯数字，不使用 `-beta.1`、`-rc.1` 等后缀。Launcher 保持独立版本，通过 `RELEASE_PROTOCOL_VERSION` 与 Core 协商。
- Web 在 `release-manifest.json` 中声明兼容的 Core 范围；Core 与 Web 均从 0.8.0 起采用新更新协议，不兼容 0.7.x 的旧 Release 格式。
- Docker keeps an independent environment version in `docker/VERSION`; bump it only when the base image, system dependencies, bundled runtime tools, or container contract changes.
- `Publish Core Pre-release` 与 `Publish Web Pre-release` 分别只构建自己的组件、创建 GitHub Pre-release，并更新 `main/release/channels/preview.json`。测试通过后，`Promote Component Release` 原地提升同一 Release 并更新 `stable.json`。
- `main/release/channels` 是面板更新与 Docker 的唯一来源：每个通道文件绑定 Core 三架构包、Web 包、Tag、兼容契约、下载地址、大小和 SHA-256。写入前会拒绝 API contract 或 Core 兼容范围不匹配的组合。
- 首次使用前运行 `Bootstrap Update Channel Index` 为 `main/release/channels/preview.json` 和 `stable.json` 各创建一次索引。索引只接受 `core-v0.8.0` 及以后完整的组件 Release；历史 `v0.8.0` 资产不完整时，应发布新的 `core-v0.8.1` / `web-v2.0.0` 基线。
- 本地 `./build.sh [amd64|arm64|armv7|all]` 仍可生成组合测试包；它不是线上通道索引或正式组件发布的来源。
- `Publish Docker Image` 由 `main/release/channels/preview.json` 或 `stable.json` 变更自动触发，也可手动选择通道。它验证索引引用的组件后构建多架构镜像，始终使用 Docker 环境版本与 `latest` 标签，并记录实际 Core/Web 版本。
- 发布和索引写入必须使用 `RELEASE_TOKEN`（fine-grained PAT，目标仓库 `Contents: Read and write`、`Workflows: Read and write`），以便写入 `main` 后可靠触发 Docker 工作流。
- Plugins: independently versioned packages; their manifests must declare their own license.
- User data and imported scripts: never part of the source distribution.
