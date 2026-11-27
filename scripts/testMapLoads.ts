// What each test file of the test map loads of the checkout's JS and TS, as
// it runs, recorded with the map (scripts/testMap.ts, #2482, ADR 1210): its
// own `bun test` process's modules and every child's it starts, Bun's and
// Node's, a recorder in each (`scripts/testMapLoads/`). A test that builds an
// app through `@rust-js/build` loads `tooling/build.js` in a child, which no
// import of the test's says. The selector runs a changed file's loaders
// (scripts/dependants.ts).
//
// Each process writes its list to the folder `RUST_JS_TEST_MAP_LOADS` names,
// the unit's, which each child inherits: that's how a child's loads are its
// test's. A child given an environment of its own that drops it is said
// (`lost-<pid>.txt`), and its test's loads are unknown, run on every change
// they'd select by. Measured on 37 files of the suite: +1.5% of the map's
// run (#2482's step 1).

import { existsSync, readdirSync, readFileSync, realpathSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { loadsEnv } from "./testMapLoads/shared.mjs";

export { loadsEnv };

/** Each recorder: the test process's, every Bun process's and every Node
 * process's. */
export const recorders = {
  test: join(import.meta.dir, "testMapLoads", "test.ts"),
  bun: join(import.meta.dir, "testMapLoads", "bun.ts"),
  node: join(import.meta.dir, "testMapLoads", "node.mjs"),
};

/** A unit's command and environment with its recorders: the test process's
 * preload after `bun test`, and every child's, Bun's by `BUN_OPTIONS`, Node's
 * by `NODE_OPTIONS`, after what the environment gives them already. */
export function recording(cmd: string[], env: Record<string, string | undefined>, dir: string): { cmd: string[]; env: Record<string, string> } {
  // Once, where a recorded test records a test of its own, the map's tests'.
  const after = (given: string | undefined, option: string) => (given?.split(" ").includes(option) ? given : [given, option].filter(Boolean).join(" "));
  const test = cmd[0] === "bun" && cmd[1] === "test" ? ["bun", "test", `--preload=${recorders.test}`, ...cmd.slice(2)] : cmd;
  return {
    cmd: test,
    env: {
      [loadsEnv]: dir,
      BUN_OPTIONS: after(env.BUN_OPTIONS, `--preload=${recorders.bun}`),
      NODE_OPTIONS: after(env.NODE_OPTIONS, `--import=${recorders.node}`),
    },
  };
}

/** Of what a unit's processes listed, a path or a `file:` URL each, the
 * checkout's files, from its root: none of `node_modules/` (a workspace
 * package linked there is its folder's, `tooling/`), of a test's own
 * target, or the recorders', nor a script given as text, `bun -e`'s `[eval]`.
 * Sorted, each once. */
export function loadedFiles(listed: string[], root: string): string[] {
  const files = new Set<string>();
  // The root as it was given and as the processes say it, macOS's `/tmp`
  // being `/private/tmp`.
  const real = realpathSync(root);
  for (const line of listed) {
    let path = line.trim();
    if (path.startsWith("file:")) path = fileURLToPath(path);
    if (path.startsWith(`${real}/`)) path = join(root, path.slice(real.length + 1));
    if (!path.startsWith(`${root}/`)) continue;
    if (path.includes("/node_modules/")) {
      try {
        path = realpathSync(path);
        if (path.startsWith(`${real}/`)) path = join(root, path.slice(real.length + 1));
      } catch {
        continue;
      }
    }
    const file = relative(root, path);
    if (file.startsWith("..") || file.includes("node_modules/") || /^(?:target|\.cache)\//.test(file) || file.startsWith("scripts/testMapLoads") || /(?:^|\/)\[[^/]*\]$/.test(file)) continue;
    files.add(file);
  }
  return [...files].sort();
}

/** What a unit's processes wrote to `dir`: the files they loaded, and, where
 * one started a child that records nothing, its command. */
export function unitLoads(dir: string, root: string): { files: string[]; lost?: string } {
  if (!existsSync(dir)) return { files: [] };
  const listed: string[] = [];
  let lost: string | undefined;
  for (const name of readdirSync(dir).sort()) {
    const text = readFileSync(join(dir, name), "utf8");
    if (name.startsWith("lost-")) lost ??= text.split("\n")[0];
    else listed.push(...text.split("\n").filter(Boolean));
  }
  return { files: loadedFiles(listed, root), ...(lost === undefined ? {} : { lost }) };
}
