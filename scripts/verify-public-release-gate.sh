#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

run() {
  printf '\n==> %s\n' "$*"
  "$@"
}

assert_no_match() {
  local description="$1"
  local pattern="$2"
  local file="$3"
  if grep -Eq "$pattern" "$file"; then
    printf '\nPublic release gate failed: %s\n' "$description" >&2
    grep -En "$pattern" "$file" >&2 || true
    exit 1
  fi
}

cleanup() {
  rm -rf "$tmp_dir"
}

assert_private_sources_absent() {
  printf '\n==> private source directories are absent\n'
  local name
  for name in niupanel-loader niupanel-agents; do
    if [ -e "$name" ]; then
      printf 'Public release gate failed: %s must not exist in the public repo.\n' "$name" >&2
      exit 1
    fi
  done
}

tmp_dir="$(mktemp -d)"
trap cleanup EXIT

run cargo fmt --check
run node scripts/verify-open-source-readiness.mjs
run node scripts/verify-source-module-size.mjs
run cargo check -p niupanel
run cargo check -p niupanel --all-features
run cargo check -p niupanel-bot --all-features
run cargo check -p niupanel-launcher
run cargo test -p niupanel public_openapi_exposes_plugins_without_legacy_agents_routes
run cargo test -p niupanel modules::system::service::tests
run cargo test -p niupanel modules::system::web_releases::tests
run cargo test -p niupanel-launcher
run cargo test -p niupanel-plugin --lib
run cargo test -p niupanel-plugin json_lines_plugin_can_call_injected_tools
run cargo test -p niupanel-plugin process_sandbox_hides_host_files_and_clears_host_environment
run cargo check --workspace
run node scripts/verify-version-contract.mjs
assert_private_sources_absent

printf '\n==> cargo tree -p niupanel -e normal --no-default-features > %s\n' "$tmp_dir/niupanel-default-tree.txt"
cargo tree -p niupanel -e normal --no-default-features > "$tmp_dir/niupanel-default-tree.txt"
assert_no_match \
  "default niupanel dependency tree must not include private loader or agents crates" \
  'niupanel-loader|niupanel_loader|niupanel-agents|niupanel_agents' \
  "$tmp_dir/niupanel-default-tree.txt"

old_private_refs="$tmp_dir/old-private-source-refs.txt"
if grep -R --exclude=verify-public-release-gate.sh -E 'niupanel-loader|niupanel_loader|niupanel-agents|niupanel_agents|legacy-loader|legacy-agents-api|niupanel_private_|VITE_ENABLE_LEGACY_AGENTS|NIUPANEL_ENABLE_LEGACY_AGENTS|src/views/modules/Agents\.vue|src/api/agents\.ts|src/types/agents\.ts' \
  Cargo.toml niupanel niupanel-bot niupanelweb docs packages scripts examples >"$old_private_refs" 2>/dev/null; then
  printf '\nPublic release gate failed: legacy private agents/loader references are still present.\n' >&2
  cat "$old_private_refs" >&2
  exit 1
fi

old_mcp_client_refs="$tmp_dir/old-mcp-client-refs.txt"
if grep -R --exclude=verify-public-release-gate.sh -E 'MCP_SERVERS|mcp\.servers|StreamableHttpClientTransport|/api/v1/mcp/servers|McpPanel\.vue|verify-agent-plugin-mcp-template' \
  niupanel niupanel-common niupanel-core niupanelweb docs packages scripts examples >"$old_mcp_client_refs" 2>/dev/null; then
  printf '\nPublic release gate failed: removed external MCP client references are still present.\n' >&2
  cat "$old_mcp_client_refs" >&2
  exit 1
fi

old_agent_paths="$tmp_dir/old-agent-plugin-paths.txt"
if grep -R -E '/api/v1/agents/plugins|agents/plugins' niupanel niupanelweb docs packages examples >"$old_agent_paths" 2>/dev/null; then
  printf '\nPublic release gate failed: old agents plugin API paths are still referenced.\n' >&2
  cat "$old_agent_paths" >&2
  exit 1
fi

old_agent_migrations="$tmp_dir/old-agent-migrations.txt"
if grep -R -n -E 'create_table\(|AgentSessions::Table|AgentMessages::Table|AgentRuns::Table|AgentMemories::Table|AgentMemoryEvents::Table|Table::drop\(\)\.table\(Agent' \
  migration/src/m/m20260618_000001_create_agent_sessions.rs \
  migration/src/m/m20260619_000001_create_agent_memories.rs \
  migration/src/m/m20260619_000002_create_agent_memory_events.rs >"$old_agent_migrations" 2>/dev/null; then
  printf '\nPublic release gate failed: private agents data tables must not be created or dropped by public migrations.\n' >&2
  cat "$old_agent_migrations" >&2
  exit 1
fi

run node scripts/package-plugin.mjs examples/plugins/compiler/echo-compiler \
  --out "$tmp_dir/plugin-packages" \
  --market "$tmp_dir/plugin-packages/index.json" \
  --download-url ./echo-compiler.tgz \
  --index-name "NiuPanel Gate Plugin Market"
run node scripts/package-plugin.mjs examples/plugins/theme-graphite \
  --out "$tmp_dir/plugin-packages" \
  --market "$tmp_dir/plugin-packages/index.json" \
  --download-url ./theme-graphite.tgz
run node -e 'const fs=require("node:fs"); JSON.parse(fs.readFileSync("docs/plugins/plugin.schema.json","utf8")); console.log("plugin schema JSON parsed.");'

run node -e 'const {generateKeyPairSync}=require("node:crypto"); const {writeFileSync}=require("node:fs"); const {privateKey}=generateKeyPairSync("ed25519"); writeFileSync(process.argv[1], privateKey.export({format:"pem", type:"pkcs8"}));' "$tmp_dir/plugin-signing-key.pem"
run node scripts/package-plugin.mjs examples/plugins/compiler/echo-compiler \
  --out "$tmp_dir/signed-plugin-packages" \
  --market "$tmp_dir/signed-plugin-packages/index.json" \
  --download-url ./echo-compiler.tgz \
  --sign-key "$tmp_dir/plugin-signing-key.pem"
run node -e 'const fs=require("node:fs"); const index=JSON.parse(fs.readFileSync(process.argv[1],"utf8")); const entry=index.plugins.find((plugin)=>plugin.id==="echo-compiler"); if (!entry?.signature_ed25519 || !entry?.public_key_ed25519 || !String(entry.public_key_ed25519).includes("BEGIN PUBLIC KEY")) { console.error("signed plugin market entry is missing signature or public key"); process.exit(1); } console.log("signed plugin market entry verified.");' "$tmp_dir/signed-plugin-packages/index.json"

run node scripts/create-private-plugin-repo.mjs "$tmp_dir/private-plugin-repo" \
  --agents-id private-agent-app \
  --compiler-id private-compiler-loader
run node -e 'const fs=require("node:fs"); const root=process.argv[1]; for (const file of ["schemas/plugin.schema.json","plugins/private-agent-app/plugin.json","plugins/private-compiler-loader/plugin.json"]) JSON.parse(fs.readFileSync(`${root}/${file}`,"utf8")); for (const file of ["plugins/private-agent-app/plugin.json","plugins/private-compiler-loader/plugin.json"]) { const manifest=JSON.parse(fs.readFileSync(`${root}/${file}`,"utf8")); if (manifest.$schema !== "../../schemas/plugin.schema.json") { console.error(`${file} has invalid $schema`); process.exit(1); } } console.log("private plugin repo schema wiring verified.");' "$tmp_dir/private-plugin-repo"
run pnpm --dir "$tmp_dir/private-plugin-repo" run signing:keygen -- keys/plugin-ed25519.pem
run node "$tmp_dir/private-plugin-repo/scripts/package-plugin.mjs" \
  "$tmp_dir/private-plugin-repo/plugins/private-compiler-loader" \
  --out "$tmp_dir/private-plugin-repo/dist/plugins" \
  --market "$tmp_dir/private-plugin-repo/dist/plugins/index.json" \
  --download-url ./private-compiler-loader.tgz
run env PLUGIN_SIGN_KEY="$tmp_dir/private-plugin-repo/keys/plugin-ed25519.pem" \
  pnpm --dir "$tmp_dir/private-plugin-repo" run package:compiler:signed
run pnpm --dir "$tmp_dir/private-plugin-repo" run package:agents

cd "$ROOT_DIR/niupanelweb"
run pnpm run verify:plugin-route-contract
run pnpm run verify:ui-design-system
run pnpm exec vue-tsc --noEmit
run rm -rf dist
run pnpm run build
run pnpm run verify:public-build

printf '\nPublic release gate passed.\n'
