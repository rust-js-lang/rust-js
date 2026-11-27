// The load recorder of a test file's own `bun test` process
// (scripts/testMapLoads.ts, #2482), its `--preload`: the modules it loaded,
// written after all its tests, as `bun test` fires no `exit`. Its Bun
// children's are `bun.ts`'s, which runs here too and writes as it goes.

import { afterAll } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";

import { loadsEnv } from "./shared.mjs";

const at = process.execArgv.findIndex((arg) => arg.startsWith("--preload=") && arg.endsWith("/scripts/testMapLoads/test.ts"));
if (at >= 0) process.execArgv.splice(at, 1);

const dir = process.env[loadsEnv];
if (dir !== undefined) {
  afterAll(() => {
    try {
      writeFileSync(join(dir, `test-${process.pid}.txt`), `${Object.keys(require.cache).join("\n")}\n`);
    } catch {}
  });
}
