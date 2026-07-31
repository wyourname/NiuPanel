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
- The formal application version is the Core version, and the Git Tag must match it. Launcher has an independent component version; Core/Launcher compatibility is enforced through `RELEASE_PROTOCOL_VERSION` in `core-release.json`.
- Web keeps its own component version and declares compatible Core versions in `release-manifest.json`.
- Docker keeps an independent environment version in `docker/VERSION`; bump it only when the base image, system dependencies, bundled runtime tools, or container contract changes.
- 推送 `v<core-version>` Tag，或手动运行 `Publish NiuPanel Release`，都会生成 GitHub Release。手动运行时，选择 `main` 分支会创建正式 Release，选择 `dev` 分支会创建 Pre-release；输入的 Tag 必须与所选分支的 Core 版本一致，且 Pre-release Tag 必须带预发布后缀，例如 `v0.8.1-dev.1`。Release 包含三架构 Core 包、Web 包和 `niupanel-release.json`，后者绑定 Core/Web 的校验和与 Git SHA。
- Docker images only package published stable Core releases from `main`. They use the environment tag `<docker-environment-version>`, with `latest` as a rolling alias; pre-release Core versions and pre-release Docker environment versions are rejected. The bundled Core version and Dockerfile source revision are recorded in OCI labels.
- Docker 镜像仅通过 `Publish Docker Image` 的手动工作流构建。输入一个已经存在的稳定 Core Release Tag；工作流固定使用 `main` 的 Dockerfile 与 `docker/VERSION`，下载并根据该 Release 的 `niupanel-release.json` 校验三架构 Core 包，再进行多架构构建。
- Plugins: independently versioned packages; their manifests must declare their own license.
- User data and imported scripts: never part of the source distribution.
