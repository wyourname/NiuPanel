#!/usr/bin/env node

import readline from "node:readline";

const pendingCalls = new Map();
const lines = readline.createInterface({
  input: process.stdin,
  crlfDelay: Infinity,
});

lines.on("line", (line) => {
  if (!line.trim()) return;
  try {
    const message = JSON.parse(line);
    if (message.type === "tool_result") {
      handleToolResult(message);
    } else {
      handleRequest(message);
    }
  } catch (error) {
    writeMessage({
      ok: false,
      error: {
        code: "invalid_request",
        message: error instanceof Error ? error.message : String(error),
      },
    });
  }
});

function handleRequest(request) {
  const input = request.input ?? {};
  const action = request.action || "summarize";

  if (action === "call_tool") {
    const tool = (request.tools ?? []).find(
      (item) => item.source === "mcp" || String(item.name ?? "").startsWith("mcp__"),
    );
    if (!tool) {
      writeMessage({
        request_id: request.request_id,
        ok: false,
        error: {
          code: "mcp_tool_unavailable",
          message: "No MCP tool is exposed to this plugin.",
        },
      });
      return;
    }

    const callId = `${request.request_id}-tool-1`;
    pendingCalls.set(callId, { request, tool });
    writeMessage({
      type: "tool_call",
      request_id: request.request_id,
      call_id: callId,
      tool: tool.name,
      input: isObject(input.tool_input) ? input.tool_input : {},
    });
    return;
  }

  const note = typeof input.note === "string" ? input.note : "";
  writeMessage({
    request_id: request.request_id,
    ok: true,
    output: {
      action,
      summary: note
        ? `Agent app template received ${note.length} characters.`
        : "Agent app template backend is reachable.",
      input,
      capabilities: request.capabilities,
      tools: request.tools ?? [],
      handled_at: new Date().toISOString(),
    },
  });
}

function handleToolResult(result) {
  const pending = pendingCalls.get(result.call_id);
  if (!pending || pending.request.request_id !== result.request_id) {
    throw new Error("Tool result does not match a pending call.");
  }
  pendingCalls.delete(result.call_id);

  writeMessage({
    request_id: result.request_id,
    ok: result.ok,
    output: result.ok
      ? {
          action: "call_tool",
          tool: pending.tool.name,
          tool_result: result.output ?? null,
          handled_at: new Date().toISOString(),
        }
      : undefined,
    error: result.ok
      ? undefined
      : result.error ?? {
          code: "mcp_tool_failed",
          message: "MCP tool call failed.",
        },
  });
}

function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function writeMessage(message) {
  process.stdout.write(`${JSON.stringify(message)}\n`);
}
