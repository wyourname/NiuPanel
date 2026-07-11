#!/usr/bin/env node

let body = "";

process.stdin.setEncoding("utf8");
process.stdin.on("data", (chunk) => {
  body += chunk;
});

process.stdin.on("end", () => {
  try {
    const request = JSON.parse(body);
    const response = {
      request_id: request.request_id,
      ok: true,
      output: {
        action: request.action,
        input: request.input,
        agent_id: request.agent_id,
        capabilities: request.capabilities,
      },
    };
    process.stdout.write(JSON.stringify(response));
  } catch (error) {
    const response = {
      ok: false,
      error: {
        code: "invalid_request",
        message: error instanceof Error ? error.message : String(error),
      },
    };
    process.stdout.write(JSON.stringify(response));
    process.exitCode = 1;
  }
});
