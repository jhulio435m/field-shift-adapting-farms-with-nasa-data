import { cp } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
await cp(".next/static", ".next/standalone/.next/static", {
  recursive: true,
  force: true,
});
process.env.NODE_ENV = "production";
process.env.NEXT_TELEMETRY_DISABLED = "1";
process.env.HOSTNAME = "127.0.0.1";
process.env.PORT ||= "3000";
await import(pathToFileURL(resolve(".next/standalone/server.js")).href);
