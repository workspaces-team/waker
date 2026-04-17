import { chmodSync } from "node:fs";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";

const scriptPath = resolve(import.meta.dirname, "npm-auth.sh");
chmodSync(scriptPath, 0o755);

const result = spawnSync(scriptPath, {
  stdio: "inherit",
});

process.exit(result.status ?? 1);
