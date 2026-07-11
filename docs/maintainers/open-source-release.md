# Open-source release checklist

This repository previously contained local runtime state and downloaded build tools. A public release must be produced from a clean Git history, not only from a clean working tree.

## Required checks

1. Confirm `LICENSE`, `SECURITY.md`, `CONTRIBUTING.md`, and `CODE_OF_CONDUCT.md` are present.
2. Confirm the README describes the project as open source and documents the Apache-2.0 license.
3. Run `bash scripts/verify-public-release-gate.sh`.
4. Inspect `git status` and `git ls-files` for runtime state, archives, private keys, and downloaded executables.
5. Run a secret scanner such as Gitleaks or TruffleHog against the complete history.
6. Generate release archives in CI; do not commit them to the source branch.

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

`uv` and `fnm` are downloaded by build scripts. Release bundles that redistribute them must include their upstream license notices. Keep version pins and checksums in build automation, and generate a third-party notice file for every binary release.

## Publishing model

- Source code: Apache License 2.0.
- GitHub releases and container images: generated from tagged commits by CI.
- Plugins: independently versioned packages; their manifests must declare their own license.
- User data and imported scripts: never part of the source distribution.
