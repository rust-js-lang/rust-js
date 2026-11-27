// The load recorder of every Node process a test of the map starts
// (scripts/testMapLoads.ts, #2482), by `NODE_OPTIONS=--import`: each module
// of the checkout's it loads, appended as it loads, so a server the test
// kills, which fires no `exit`, has said what it loaded; a Vite config's
// bundle by what it was made of. Not under Bun, whose own recorder is
// `bun.ts`.

import childProcess from "node:child_process";
import { appendFileSync } from "node:fs";
import { registerHooks, syncBuiltinESMExports } from "node:module";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { dropsRecording, loadsEnv, viteSources } from "./shared.mjs";

const dir = process.env[loadsEnv];
const root = process.env.RUST_JS_TEST_MAP_ROOT;
if (dir !== undefined && root !== undefined && process.versions.bun === undefined) {
  const own = join(dir, `node-${process.pid}.txt`);
  const say = (paths) => {
    try {
      appendFileSync(own, `${paths.join("\n")}\n`);
    } catch {}
  };
  const seen = new Set();
  registerHooks({
    load(url, context, next) {
      const loaded = next(url, context);
      if (!seen.has(url) && url.startsWith("file:")) {
        seen.add(url);
        const path = fileURLToPath(url);
        if (path.includes("/node_modules/.vite-temp/")) say(viteSources(path, String(loaded.source ?? "")));
        else if (path.startsWith(`${root}/`) && !path.includes("/node_modules/")) say([path]);
      }
      return loaded;
    },
  });
  // A child given an environment that drops the recording's folder.
  const lost = (command, options) => {
    if (!dropsRecording(command, options?.env)) return;
    try {
      appendFileSync(join(dir, `lost-${process.pid}.txt`), `${command}\n`);
    } catch {}
  };
  for (const name of ["spawn", "spawnSync", "execFile", "execFileSync", "fork"]) {
    const original = childProcess[name];
    childProcess[name] = function (command, args, options) {
      lost(name === "fork" ? process.execPath : command, Array.isArray(args) ? options : args);
      return original.apply(this, arguments);
    };
  }
  for (const name of ["exec", "execSync"]) {
    const original = childProcess[name];
    childProcess[name] = function (command, options) {
      lost(command, typeof options === "object" ? options : undefined);
      return original.apply(this, arguments);
    };
  }
  syncBuiltinESMExports();
}
