# Echo Compiler Plugin

This is a minimal `compiler` extension plugin for the NiuPanel process runtime.

Install it from the extension manager using the server-side directory path:

```text
examples/plugins/compiler/echo-compiler
```

Supported actions:

- `versions`: returns `{ "versions": ["3.10", "3.11", "3.12"] }`.
- `encrypt`: returns `{ "file_content": "..." }`; the panel writes this content into `data/scripts/compile`.

This example provides only the compiler process runtime and does not include a
native UI.
