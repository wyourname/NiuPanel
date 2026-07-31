# Contributing to NiuPanel

Thank you for helping improve NiuPanel. Changes should stay focused, preserve existing public APIs where possible, and include verification appropriate to their risk.

## Development setup

The recommended environment is the repository Docker setup:

```bash
./start.sh
```

This starts the backend on `127.0.0.1:7788` and the Vite frontend on `127.0.0.1:7787` without writing language caches into the host checkout.

For a local toolchain, install a current stable Rust toolchain, Node.js `20.19+` or `22.12+`, and npm.

## Repository areas

- `niupanel/`: HTTP API and application composition.
- `niupanel-core/`: scheduling, execution, runtimes, and settings primitives.
- `niupanel-common/`, `niupanel-entity/`, `migration/`: shared types, persistence entities, and migrations.
- `niupanel-plugin/`, `packages/plugin-sdk/`, `examples/plugins/`: plugin runtime, TypeScript SDK, and examples.
- `niupanel-launcher/`: version activation, health checks, and rollback.
- `niupanelweb/`: Vue web and mobile UI.
- `docs/`: architecture, integrations, frontend, plugin, and maintainer documentation.

See [Repository layout](docs/architecture/repository-layout.md) for the complete boundary rules.

## Before submitting

Run the checks relevant to your change:

```bash
cargo fmt --check
cargo check --workspace
cargo test --workspace
npm --prefix niupanelweb exec vue-tsc -- --noEmit
npm --prefix niupanelweb run build
npm --prefix niupanelweb run verify:ui-design-system
```

Maintainers can run the complete release gate with:

```bash
bash scripts/verify-public-release-gate.sh
```

## Pull requests

- Explain the user-visible outcome and important tradeoffs.
- Keep generated output, runtime data, downloaded tools, databases, logs, credentials, and private keys out of commits.
- Add or update tests when behavior changes.
- Update documentation when configuration, APIs, migrations, plugin contracts, or deployment behavior changes.
- Do not bundle unrelated formatting or refactors with a behavioral fix.

By contributing, you agree that your contribution is licensed under the Apache License 2.0 used by this repository.
