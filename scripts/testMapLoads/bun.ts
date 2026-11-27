// The load recorder of every Bun process a test of the map starts
// (scripts/testMapLoads.ts, #2482), preloaded by `BUN_OPTIONS`: the modules
// it loaded, `require.cache`'s keys, ESM's too, with what each Vite config
// bundle it imported was made of, written to the test's folder as it runs
// and as it exits. A test kills the servers it starts, and a signal fires no
// `exit`: so the list is written again each second it grew, the timer
// holding nothing open. Not of the map's own compiler, a process each
// compile, which loads nothing of a test's.

import { appendFileSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { dropsRecording, loadsEnv, viteSources } from "./shared.mjs";

const dir = process.env[loadsEnv];
// Out of what a fork is given: Next.js's dev server starts its workers
// with its own `execArgv`, and with a preload there its page never
// rendered (#2482's measurement, `next dev`).
const recorder = (arg: string) => arg.startsWith("--preload=") && arg.endsWith("/scripts/testMapLoads/bun.ts");
for (let at = process.execArgv.findIndex(recorder); at >= 0; at = process.execArgv.findIndex(recorder)) process.execArgv.splice(at, 1);

if (dir !== undefined && !Bun.main.endsWith("/scripts/testMapCompiler.ts")) {
  const own = join(dir, `bun-${process.pid}.txt`);
  const bundled = new Set<string>();
  let written = -1;
  const write = () => {
    const loaded = [Bun.main, ...Object.keys(require.cache), ...bundled];
    if (loaded.length === written) return;
    written = loaded.length;
    try {
      writeFileSync(own, `${loaded.join("\n")}\n`);
    } catch {}
  };
  // Vite imports a config's bundle and removes it, so what it was made of
  // is read as it loads, the same text given back.
  Bun.plugin({
    name: "test-map-loads",
    setup(build) {
      build.onLoad({ filter: /\/node_modules\/\.vite-temp\/[^/]+\.mjs$/ }, ({ path }) => {
        const contents = readFileSync(path, "utf8");
        for (const source of viteSources(path, contents)) bundled.add(source);
        write();
        return { contents, loader: "js" };
      });
    },
  });
  // A child given an environment that drops the recording's folder: what it
  // loads is no list's, so the test's are unknown.
  const lost = (command: unknown, env: Record<string, string | undefined> | undefined) => {
    const named = Array.isArray(command) ? String(command[0]) : String(command);
    if (!dropsRecording(named, env)) return;
    try {
      appendFileSync(join(dir, `lost-${process.pid}.txt`), `${[command].flat().join(" ")}\n`);
    } catch {}
  };
  for (const name of ["spawn", "spawnSync"] as const) {
    const spawn = Bun[name] as (...args: any[]) => any;
    (Bun as any)[name] = (first: any, options?: any) => {
      if (Array.isArray(first)) lost(first, options?.env);
      else if (first !== null && typeof first === "object") lost(first.cmd, first.env);
      return options === undefined ? spawn(first) : spawn(first, options);
    };
  }
  setInterval(write, 1000).unref();
  process.on("exit", write);
}
