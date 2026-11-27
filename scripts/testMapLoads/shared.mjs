// What every process's load recorder shares (scripts/testMapLoads.ts, #2482):
// the variable that names where a test's processes write what they loaded,
// what a bundled Vite config was bundled from, and whether a child is started
// where no recorder can follow it. Plain JS, so Node's recorder imports it as
// Bun's do, and nothing of the checkout's but itself.

import { dirname, resolve } from "node:path";

/** The folder a test's processes each write their loads to, one file a
 * process, which every child the test starts inherits. */
export const loadsEnv = "RUST_JS_TEST_MAP_LOADS";

/** Vite's bundle of a config, `node_modules/.vite-temp/<name>.timestamp-….mjs`,
 * which it imports and removes, so its own name is no file of the checkout's. */
const viteTemp = /\/node_modules\/\.vite-temp\/[^/]+\.mjs$/;

/** The files a Vite config's bundle at `path` was made of, by the inline
 * source map Vite gives it: `browser/vitest.config.ts` for its
 * `vitest.config.ts.timestamp-….mjs`. None of another file. */
export function viteSources(path, contents) {
  if (!viteTemp.test(path)) return [];
  const map = /\/\/# sourceMappingURL=data:application\/json[^,]*;base64,(\S+)\s*$/.exec(contents)?.[1];
  if (map === undefined) return [];
  try {
    const sources = JSON.parse(Buffer.from(map, "base64").toString("utf8")).sources ?? [];
    return sources.filter((source) => typeof source === "string").map((source) => resolve(dirname(path), source.replace(/^file:\/\//, "")));
  } catch {
    return [];
  }
}

/** The JS runtimes a test starts, by the command's name: a child of one of
 * these whose environment drops `loadsEnv` loads what no list says. */
const runtimes = /^(?:bun|bunx|node|npx|npm|vite|vitest|next|tsx)$|\.(?:[cm]?js|[cm]?ts)$/;

/** Whether starting `command` with `env` loses what it loads: a JS runtime
 * given an environment of its own that doesn't name where to write. An
 * inherited one (`env` undefined) names it. */
export function dropsRecording(command, env) {
  if (env === undefined || env === null || typeof command !== "string") return false;
  if (typeof env[loadsEnv] === "string") return false;
  const name = command.trim().split(/\s+/)[0]?.split("/").pop() ?? "";
  return runtimes.test(name);
}
