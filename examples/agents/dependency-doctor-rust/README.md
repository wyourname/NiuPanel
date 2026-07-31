# Dependency Doctor

Rust process Agent for diagnosing Node.js and Python dependency resolution failures.

## Development Install

Install this directory as a local Agent plugin from the NiuPanel Agent page:

```text
examples/agents/dependency-doctor-rust
```

The development entry is `run.sh`, which runs:

```bash
cargo run --quiet --manifest-path Cargo.toml
```

## Production Package

Build a binary and point `plugin.json` at it:

```bash
cargo build --release
cp target/release/dependency-doctor-rust ./dependency-doctor-rust
```

Then change:

```json
"entry": "dependency-doctor-rust"
```

## Invoke Example

```json
{
  "action": "analyze_dependencies",
  "input": {
    "env_type": "Nodejs",
    "requirements": "",
    "log": "Error: Cannot find module 'axios'\nRequire stack:\n- /workspace/data/scripts/node/test_node_script.js",
    "cwd": "/workspace/data/scripts/node"
  }
}
```

The Agent returns a structured diagnosis with `summary`, `severity`, `findings`, `recommendations`, and `next_actions`.
