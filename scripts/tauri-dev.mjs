#!/usr/bin/env node
// `npm run tauri:dev` with an optional dev-server port override.
//
// The Vite port is read from VITE_DEV_PORT (default 5173) in vite.config.ts; Tauri's
// `build.devUrl` is static JSON, so this wrapper merges a matching override via `--config`.
// Usage: VITE_DEV_PORT=5174 npm run tauri:dev
import { spawnSync } from "node:child_process";
import process from "node:process";

const port = process.env.VITE_DEV_PORT;
const args = ["tauri", "dev"];
if (port) {
  if (!/^\d+$/.test(port)) {
    console.error(`VITE_DEV_PORT must be a number, got ${JSON.stringify(port)}`);
    process.exit(1);
  }
  args.push("--config", JSON.stringify({ build: { devUrl: `http://localhost:${port}` } }));
}
args.push(...process.argv.slice(2));

const result = spawnSync("npx", args, { stdio: "inherit", env: process.env });
process.exit(result.status ?? 1);
