# Repository layout

NiuPanel is a Cargo workspace with a Vue application and a separate plugin ecosystem. The Rust crates intentionally remain at the repository root: their names are public package and build-script identifiers, and moving them into a cosmetic `crates/` directory would add churn without improving ownership.

## Runtime applications

| Path | Responsibility |
| --- | --- |
| `niupanel/` | Axum API, application startup, HTTP modules, and service composition. |
| `niupanel-launcher/` | Core activation, health checks, release switching, and rollback. |
| `niupanel-bot/` | 过渡期内置 Telegram 实现，计划迁移为独立插件。 |
| `niupanel-proxy/` | Shared proxy and transport support. |
| `niupanelweb/` | Vue 3 web UI and Capacitor mobile wrapper. |

## Shared Rust crates

| Path | Responsibility |
| --- | --- |
| `niupanel-core/` | Task scheduling, execution, runtime management, settings, and system primitives. |
| `niupanel-common/` | Configuration, authentication primitives, common models, logging, and version contracts. |
| `niupanel-entity/` | SeaORM entities. |
| `migration/` | Ordered database schema migrations. |
| `niupanel-sdk/` | Runtime SDK interfaces exposed to scripts. |

## Plugin ecosystem

| Path | Responsibility |
| --- | --- |
| `niupanel-plugin/` | Plugin manifests, packaging, process runtime, sandboxing, and host tools. |
| `packages/plugin-sdk/` | TypeScript SDK for plugin UI applications. |
| `examples/plugins/` | Minimal public plugin examples and marketplace fixtures. |
| `examples/agents/` | Process-plugin examples using the agent protocol. |
| `docs/plugins/` | Plugin schemas and authoring guides. |

## Operations and documentation

| Path | Responsibility |
| --- | --- |
| `docker/` | Dockerfiles and Docker packaging scripts. Generated archives do not belong in Git. |
| `magisk/` | Magisk module templates and download/build scripts. Downloaded tool binaries do not belong in Git. |
| `scripts/` | Release, packaging, signing, and repository verification commands. |
| `docs/` | Architecture, integration, frontend, plugin, and maintainer documentation. |
| `data/` | Local runtime state. This directory is created at runtime and must never be committed. |
| `release_tools/` | Local downloaded build-tool cache. This directory must never be committed. |

## Boundary rules

1. HTTP handlers should delegate business logic to services or core crates.
2. `niupanel-core` must not depend on web UI implementation details.
3. Public plugin contracts belong in `niupanel-plugin`, `packages/plugin-sdk`, or `docs/plugins`, not in private application modules.
4. Vue pages should compose focused components and composables; API calls belong in `src/api`, shared state in `src/stores`, and reusable behavior in `src/composables`.
5. Generated output, downloaded binaries, databases, user scripts, logs, backups, and credentials are repository-external state.
6. `examples/` contains executable fixtures and copyable starter projects used by build and release checks; prose-only guidance belongs in `docs/`.
