const EXT_LANGUAGE_MAP: Record<string, string> = {
  js: "javascript",
  mjs: "javascript",
  cjs: "javascript",
  ts: "typescript",
  tsx: "typescript",
  jsx: "javascript",
  py: "python",
  pyw: "python",
  sh: "shell",
  bash: "shell",
  zsh: "shell",
  fish: "shell",
  json: "json",
  jsonc: "json",
  html: "html",
  htm: "html",
  css: "css",
  scss: "scss",
  less: "less",
  vue: "html",
  svelte: "html",
  yaml: "yaml",
  yml: "yaml",
  toml: "toml",
  md: "markdown",
  markdown: "markdown",
  rs: "rust",
  go: "go",
  sql: "sql",
  xml: "xml",
  svg: "xml",
  dockerfile: "dockerfile",
  ini: "ini",
  conf: "ini",
  env: "ini",
  log: "plaintext",
  txt: "plaintext",
  lock: "json",
  gitignore: "plaintext",
};

export type LanguageConfig = {
  tabSize: number;
  insertSpaces: boolean;
  commentPrefix: string;
  defaultSnippet: string;
};

const DEFAULT_LANGUAGE_CONFIG: LanguageConfig = {
  tabSize: 2,
  insertSpaces: true,
  commentPrefix: "#",
  defaultSnippet: "",
};

const LANGUAGE_CONFIGS: Record<string, LanguageConfig> = {
  python: {
    tabSize: 4,
    insertSpaces: true,
    commentPrefix: "#",
    defaultSnippet: `#!/usr/bin/env python3
# -*- coding: utf-8 -*-

def main():
    pass

if __name__ == "__main__":
    main()
`,
  },
  javascript: {
    tabSize: 2,
    insertSpaces: true,
    commentPrefix: "//",
    defaultSnippet: `#!/usr/bin/env node

function main() {
  console.log("Hello, World!");
}

main();
`,
  },
  typescript: {
    tabSize: 2,
    insertSpaces: true,
    commentPrefix: "//",
    defaultSnippet: `#!/usr/bin/env ts-node

function main(): void {
  console.log("Hello, World!");
}

main();
`,
  },
  shell: {
    tabSize: 2,
    insertSpaces: true,
    commentPrefix: "#",
    defaultSnippet: `#!/bin/bash

main() {
  echo "Hello, World!"
}

main
`,
  },
  json: {
    tabSize: 2,
    insertSpaces: true,
    commentPrefix: "//",
    defaultSnippet: `{
  "key": "value"
}
`,
  },
  yaml: {
    tabSize: 2,
    insertSpaces: true,
    commentPrefix: "#",
    defaultSnippet: `# YAML configuration
key: value
`,
  },
  html: {
    tabSize: 2,
    insertSpaces: true,
    commentPrefix: "<!--",
    defaultSnippet: `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>Title</title>
</head>
<body>
</body>
</html>
`,
  },
  css: {
    tabSize: 2,
    insertSpaces: true,
    commentPrefix: "/*",
    defaultSnippet: `/* Styles */
.selector {
  property: value;
}
`,
  },
  rust: {
    tabSize: 4,
    insertSpaces: true,
    commentPrefix: "//",
    defaultSnippet: `fn main() {
    println!("Hello, World!");
}
`,
  },
  go: {
    tabSize: 4,
    insertSpaces: true,
    commentPrefix: "//",
    defaultSnippet: `package main

import "fmt"

func main() {
    fmt.Println("Hello, World!")
}
`,
  },
  sql: {
    tabSize: 2,
    insertSpaces: true,
    commentPrefix: "--",
    defaultSnippet: `SELECT * FROM table_name;
`,
  },
  dockerfile: {
    tabSize: 2,
    insertSpaces: true,
    commentPrefix: "#",
    defaultSnippet: `FROM ubuntu:latest

RUN apt-get update

CMD ["bash"]
`,
  },
  ini: {
    tabSize: 2,
    insertSpaces: true,
    commentPrefix: "#",
    defaultSnippet: `# Configuration
[section]
key=value
`,
  },
  markdown: {
    tabSize: 2,
    insertSpaces: true,
    commentPrefix: "<!--",
    defaultSnippet: `# Title

Content here.
`,
  },
  toml: {
    tabSize: 2,
    insertSpaces: true,
    commentPrefix: "#",
    defaultSnippet: `# TOML configuration
[section]
key = "value"
`,
  },
};

export function getLanguageFromFilename(filename: string): string {
  if (!filename) return "plaintext";

  const lower = filename.toLowerCase();

  if (lower === "dockerfile" || lower.startsWith("dockerfile.")) {
    return "dockerfile";
  }
  if (lower === "makefile") return "plaintext";
  if (lower === ".gitignore" || lower === ".env" || lower === ".env.local") {
    return "ini";
  }
  if (lower.endsWith(".env.local") || lower.endsWith(".env.production")) {
    return "ini";
  }

  const ext = filename.split(".").pop()?.toLowerCase() || "";
  return EXT_LANGUAGE_MAP[ext] || "plaintext";
}

export function getLanguageConfig(language: string): LanguageConfig {
  return LANGUAGE_CONFIGS[language] || DEFAULT_LANGUAGE_CONFIG;
}

export function getEditorOptionsForLanguage(language: string, isMobile: boolean) {
  const config = getLanguageConfig(language);
  return {
    tabSize: config.tabSize,
    insertSpaces: config.insertSpaces,
    fontSize: isMobile ? 14 : 14,
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
    wordWrap: "on" as const,
    automaticLayout: true,
    lineNumbers: "on" as const,
    lineNumbersMinChars: isMobile ? 3 : 5,
    padding: { top: isMobile ? 8 : 12 },
    folding: !isMobile,
    glyphMargin: false,
    renderLineHighlight: "line" as const,
    cursorBlinking: "smooth" as const,
    smoothScrolling: true,
    fontFamily: "'JetBrains Mono', 'Fira Code', 'Consolas', 'Monaco', monospace",
  };
}
