# Task Ops Doctor

Rust process Agent that diagnoses NiuPanel task failures through the JSON-lines tool protocol.

It expects:

```json
{
  "action": "diagnose",
  "input": {
    "task_id": 123
  }
}
```

The Agent calls these NiuPanel tools through stdout/stdin:

- `read_task`
- `read_recent_runs`
- `read_log_tail`
- `read_node_settings`
- `read_python_settings`

It returns a structured diagnosis:

- `summary`
- `severity`
- `confidence`
- `root_causes`
- `findings`
- `recommendations`
- `next_actions`
- `safe_to_auto_fix`

## Development Install

Install this directory as a local Agent plugin:

```text
/workspace/examples/agents/task-ops-doctor-rust
```

The development entry is `run.sh`, which runs `cargo run`.

## Production Package

Build a standalone binary:

```bash
cargo build --release
cp target/release/task-ops-doctor-rust ./task-ops-doctor-rust
```

Then change `plugin.json`:

```json
"entry": "task-ops-doctor-rust"
```
