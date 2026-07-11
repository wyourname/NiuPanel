#!/usr/bin/env node
import { createHash, createPublicKey, generateKeyPairSync } from "node:crypto";
import { existsSync } from "node:fs";
import { chmod, mkdir, writeFile } from "node:fs/promises";
import path from "node:path";

const usage = `
Usage:
  node scripts/generate-plugin-signing-key.mjs [out-file] [--force]

Default:
  keys/plugin-ed25519.pem
`.trim();

const args = process.argv.slice(2);
if (args.includes("-h") || args.includes("--help")) {
  console.log(usage);
  process.exit(0);
}

const force = args.includes("--force");
const outArg = args.find((arg) => !arg.startsWith("--"));
const outFile = path.resolve(outArg ?? "keys/plugin-ed25519.pem");

if (existsSync(outFile) && !force) {
  fail(`Signing key already exists: ${outFile}. Use --force to replace it.`);
}

const { privateKey } = generateKeyPairSync("ed25519");
const privatePem = privateKey.export({ format: "pem", type: "pkcs8" }).toString();
const publicKey = createPublicKey(privateKey);
const publicPem = publicKey.export({ format: "pem", type: "spki" }).toString().trim();
const publicDer = publicKey.export({ format: "der", type: "spki" });
const rawPublicKey = Buffer.from(publicDer).subarray(-32);
const trustedKey = `sha256:${createHash("sha256").update(rawPublicKey).digest("hex")}`;

await mkdir(path.dirname(outFile), { recursive: true });
await writeFile(outFile, privatePem, { mode: 0o600 });
await chmod(outFile, 0o600);

console.log(
  JSON.stringify(
    {
      ok: true,
      private_key: outFile,
      trusted_key: trustedKey,
      public_key_ed25519: publicPem,
      env: {
        PLUGIN_SIGN_KEY: outFile,
        TRUSTED_PLUGIN_PUBLIC_KEYS: trustedKey,
      },
    },
    null,
    2,
  ),
);

function fail(message) {
  console.error(message);
  process.exit(1);
}
