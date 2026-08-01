# Open-source release checklist

This repository previously contained local runtime state and downloaded build tools. A public release must be produced from a clean Git history, not only from a clean working tree.

## Required checks

1. Confirm `LICENSE`, `SECURITY.md`, `CONTRIBUTING.md`, and `CODE_OF_CONDUCT.md` are present.
2. Confirm the README describes the project as open source and documents the Apache-2.0 license.
3. Run `bash scripts/verify-public-release-gate.sh`.
4. Run `node scripts/verify-version-contract.mjs v<core-version>` and ensure the release tag matches `niupanel/Cargo.toml`.
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
- The formal application version is the Core version, and the Git Tag must match it. Application versions and Tags always use the plain `X.Y.Z` / `vX.Y.Z` form; do not add `-beta.1`, `-rc.1`, or another prerelease suffix. Launcher has an independent component version; Core/Launcher compatibility is enforced through `RELEASE_PROTOCOL_VERSION` in `core-release.json`.
- Web keeps its own component version and declares compatible Core versions in `release-manifest.json`.
- Docker keeps an independent environment version in `docker/VERSION`; bump it only when the base image, system dependencies, bundled runtime tools, or container contract changes.
- 推送 `v<core-version>` Tag，或手动运行 `Publish NiuPanel Release` 并选择 `publish-prerelease`，只构建一次三架构 Core 包和 Web 包，然后创建 GitHub Pre-release。测试通过后，对同一 Tag 手动选择 `promote-stable`；工作流会重新下载并校验原资产，再只修改同一个 GitHub Release 的 Pre-release 状态，不重建、不重新打包，也不替换资产。
- 每个应用 Release 都必须包含三个 Core 归档、唯一的 Web 归档和 schema 2 的 `niupanel-release.json`。该索引绑定 Git SHA、组件兼容契约、文件名、架构、大小和 SHA-256，不持久化 preview/stable 通道；通道只由 GitHub Release 的 `prerelease` 状态决定。内置更新器必须先读取并验证该索引，不能通过文件名猜测 Core 或 Web 资产。
- 本地 `./build.sh [amd64|arm64|armv7|all]` 生成与 CI 相同结构的 Core/Web 归档和 `docker/niupanel-release.json`，并在构建结束时调用同一个 bundle verifier。它允许只包含本次构建的单架构 Core；CI 发布则强制包含全部三架构。
- Docker images can package any published application Release, including a GitHub Pre-release, after verifying its `niupanel-release.json`. They always use the environment tag `<docker-environment-version>` and the rolling `latest` alias; the bundled Core and Web versions are recorded in OCI labels.
- `Publish Docker Image` 会在应用 Release 发布为 Pre-release 或正式版时自动构建 Docker 镜像，也可手动输入一个已经发布的应用 Release Tag。工作流固定使用 `main` 的 Dockerfile 与 `docker/VERSION`，下载并根据该 Release 的同一份 `niupanel-release.json` 校验三架构 Core 包和唯一的 Web 包，再进行多架构构建，沿用环境版本标签和 `latest`。
- 发布步骤优先使用 `RELEASE_TOKEN`（fine-grained PAT，目标仓库 `Contents: Read and write`），未配置时才使用 `GITHUB_TOKEN`；若使用后者，仓库 Settings → Actions → General → Workflow permissions 必须设为 `Read and write permissions`。
- Plugins: independently versioned packages; their manifests must declare their own license.
- User data and imported scripts: never part of the source distribution.
